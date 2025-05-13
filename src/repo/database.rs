use sqlx::migrate::MigrateError;
use sqlx::migrate::Migrator;
use sqlx::postgres::types::PgInterval;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::Pool;
use sqlx::Postgres;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

#[derive(Clone, Debug)]
pub struct Guild {
    pub id: i64,
    pub channel: i64,
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub id: i64,
    pub guild_id: i64,
    pub user_id: i64,
    pub channel_id: i64,
    pub task: String,
    pub task_secondary: String,
    pub praise: String,
    pub praise_name: String,
    pub interval: PgInterval,
    pub created: DateTime<Utc>,
    pub next_run: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: i64,
    pub guild_id: i64,
    pub user_id: i64,
    pub task: String,
    pub task_secondary: String,
    pub interval: PgInterval,
    pub created: DateTime<Utc>,
    pub next_run: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct UserTask {
    pub id: i64,
    pub guild_id: i64,
    pub user_id: i64,
    pub task: String,
    pub task_secondary: String,
    pub interval: PgInterval,
    pub created: DateTime<Utc>,
    pub next_run: DateTime<Utc>,
    pub timezone: PgInterval,
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub guild_id: i64,
    pub user_id: i64,
    pub praise: String,
    pub praise_name: String,
    pub timezone: PgInterval,
}

#[derive(Clone, Debug)]
pub struct Timezone {
    timezone: i16,
}

#[derive(Debug)]
pub enum DatabaseErrors {
    Error,
    CannotConect,
    MigrationFolderDoesNotExist,
    MigrationError(MigrateError),
    DoesNotExist,
    GuildDoesNotExist,
    UserDoesNotExist,
    UserAlreadyExists,
}
#[derive(Clone)]
pub struct Database {
    db: Pool<Postgres>,
}

impl Database {
    pub async fn new(database_url: String) -> Result<Database, DatabaseErrors> {

        let sql_pool = match PgPool::connect(&database_url).await {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::CannotConect),
        };

        let migrator = sqlx::migrate!();

        match migrator.run(&sql_pool).await {
            Ok(_) => return Ok(Database { db: sql_pool }),
            Err(e) => return Err(DatabaseErrors::MigrationError(e)),
        };
    }

    pub async fn get_guild(&self, guild: &i64) -> Result<Option<Guild>, DatabaseErrors> {
        let opt = match sqlx::query!("SELECT * FROM guilds WHERE guildID = $1", guild)
            .fetch_optional(&self.db)
            .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        match opt {
            Some(e) => {
                return Ok(Some(Guild {
                    id: e.guildid.clone(),
                    channel: e.channel.clone(),
                }));
            }
            None => return Ok(None),
        };
    }
    pub async fn update_guild(&self, guild: &Guild) -> Result<Guild, DatabaseErrors> {
        let opt = match self.get_guild(&guild.id).await {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match opt {
            Some(e) => {
                match sqlx::query!(
                    "UPDATE guilds SET channel = $2 WHERE guildID = $1",
                    &guild.id,
                    &guild.channel,
                )
                .execute(&self.db)
                .await
                {
                    Ok(_) => {
                        return Ok(guild.clone());
                    }
                    Err(_) => return Err(DatabaseErrors::Error),
                };
            }
            None => {
                match sqlx::query!(
                    "INSERT INTO guilds (guildID, channel) VALUES($1, $2)",
                    &guild.id,
                    &guild.channel,
                )
                .execute(&self.db)
                .await
                {
                    Ok(_) => {
                        return Ok(guild.clone());
                    }
                    Err(_) => return Err(DatabaseErrors::Error),
                };
            }
        }
    }

    pub async fn get_task_id(&self, id: &i64) -> Result<Option<Task>, DatabaseErrors> {
        let opt = match sqlx::query!("SELECT s.id, s.guildid, s.userid, s.task, s.tasksecondary, s.interval, s.created, s.nextrun FROM public.schedule s INNER JOIN users u on s.userid = u.id AND s.id = $1", id)
            .fetch_optional(&self.db)
            .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        match opt {
            Some(e) => {
                return Ok(Some(Task {
                    id: e.id.clone(),
                    guild_id: e.guildid.clone(),
                    user_id: e.userid.clone(),
                    interval: e.interval.clone(),
                    next_run: e.nextrun.clone().and_utc(),
                    created: e.created.clone().and_utc(),
                    task: e.task.clone(),
                    task_secondary: e.tasksecondary.clone(),
                }));
            }
            None => return Ok(None),
        };
    }

    pub async fn get_task_user(
        &self,
        guild_id: &i64,
        user_id: &i64,
    ) -> Result<Vec<UserTask>, DatabaseErrors> {
        let tasks = match sqlx::query!(
            "SELECT s.id, u.guildid, u.userid, s.task, s.tasksecondary, s.interval, s.created, s.nextrun, u.timezone FROM schedule s INNER JOIN users u  on s.userid = u.id AND u.guildid = $1 and u.userid = $2",
            guild_id,
            user_id
        )
        .fetch_all(&self.db)
        .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        return Ok(tasks
            .iter()
            .map(|e| UserTask {
                id: e.id.clone(),
                guild_id: e.guildid.clone(),
                user_id: e.userid.clone(),
                interval: e
                    .interval
                    .clone(),
                next_run: e
                    .nextrun.and_utc()
                    .clone(),
                created: e
                    .created
                    .and_utc(),
                task: e.task.clone(),
                task_secondary: e
                    .tasksecondary
                    .clone(),
                timezone: e.timezone.clone()
            })
            .collect());
    }

    pub async fn get_task_nextrun(
        &self,
        datetime: Option<DateTime<Utc>>,
    ) -> Result<Vec<Schedule>, DatabaseErrors> {
        let datetime: DateTime<Utc> = match datetime {
            Some(e) => e,
            None => Utc::now(),
        };

        let tasks = match sqlx::query!(
            "SELECT s.id, s.guildid, g.channel, u.userid, s.task, s.tasksecondary, u.praise, u.praisename, s.interval, s.created, s.nextrun FROM public.schedule s INNER JOIN users u on s.userid = u.id AND s.nextrun < $1 INNER JOIN guilds g on s.guildid = g.guildid",
            datetime.naive_utc()
        )
        .fetch_all(&self.db)
        .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        return Ok(tasks
            .iter()
            .map(|e| Schedule {
                id: e.id.clone(),
                guild_id: e.guildid.clone(),
                user_id: e.userid.clone(),
                channel_id: e.channel.clone(),
                interval: e.interval.clone(),
                next_run: e.nextrun.clone().and_utc(),
                created: e.created.clone().and_utc(),
                praise: e.praise.clone(),
                praise_name: e
                    .praisename
                    .clone(),
                task: e.task.clone(),
                task_secondary: e.tasksecondary.clone(),
            })
            .collect());
    }

    pub async fn get_task_guild(&self, guild_id: &i64) -> Result<Vec<UserTask>, DatabaseErrors> {

        let users = match self.get_users_guild(guild_id).await{
            Ok(e) => e,
            Err(e) => return Err(e)
        };

        println!("Found {} users", users.len());

        let mut res: Vec<UserTask> = Vec::new();

        for user in users {
            let tasks = match self.get_task_user(guild_id, &user.user_id).await{
                Ok(e) => e,
                Err(e) => return Err(e)
            };

            println!("Found {} tasks", tasks.len());

            for task in tasks {
                res.push(task);
            }
        }

        return Ok(res);
    }

    pub async fn update_task(
        &self,
        id: &i64,
        schedule: Schedule,
    ) -> Result<Schedule, DatabaseErrors> {
        todo!()
    }

    pub async fn add_task(
        &self,
        schedule: Task,
    ) -> Result<Task, DatabaseErrors> {
        let user = match self.get_user_id(&schedule.user_id).await {
            Ok(e) => e,
            Err(e) => {
                println!("{:?}", e);
                return Err(e);
            }
        
        };

        match user {
            Some(_) => {
                match sqlx::query!("INSERT INTO schedule(guildid, userid, task, tasksecondary, interval, nextrun) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id", schedule.guild_id, schedule.user_id, schedule.task, schedule.task_secondary, schedule.interval, schedule.next_run.naive_utc()).fetch_one(&self.db).await{
                    Ok(e) => {
                        return Ok(Task{
                            id: e.id,
                            .. schedule
                        })
                    }, 
                    Err(e) => { 
                        println!("{:?}", e);
                        return Err(DatabaseErrors::Error);
                    }

                };

            },
            None => return Err(DatabaseErrors::UserDoesNotExist)
        }
    }

    pub async fn delete_task(&self, id: &i64) -> Result<(), DatabaseErrors> {
        match sqlx::query!("DELETE FROM schedule WHERE id = $1", id).execute(&self.db).await {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseErrors::Error)
        }
    }

    pub async fn delete_task_user(&self, guild_id: &i64, user_id: &i64) -> Result<(), DatabaseErrors> {
        match sqlx::query!("DELETE FROM schedule WHERE guildid = $1 AND userid = $2", guild_id, user_id).execute(&self.db).await {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseErrors::Error)
        }
    }

    pub async fn incriment_task(&self, id: &i64) -> Result<(), DatabaseErrors> {
        let opt = match sqlx::query!("SELECT * FROM schedule WHERE id = $1", id)
            .fetch_optional(&self.db)
            .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        match opt {
            Some(_) => {
                match sqlx::query!(
                    "UPDATE schedule SET nextrun = (nextrun + interval) WHERE id = $1",
                    id
                )
                .execute(&self.db)
                .await
                {
                    Ok(_) => return Ok(()),
                    Err(_) => return Err(DatabaseErrors::Error),
                }
            }
            None => return Ok(()),
        };
    }
    
    /// Shifts all schedules for a user by an interval
    pub async fn shift_schedules(&self, guild_id: &i64, user_id: &i64, interval: &PgInterval) -> Result<(), DatabaseErrors> {
        let uid = match self.get_user_guild(guild_id, user_id).await {
            Ok(o) => match o {
                Some(e) => e,
                None => return Ok(())
            },
            Err(_) => return Err(DatabaseErrors::Error),

        };

        println!("Running query");
        match sqlx::query!(
                "UPDATE schedule SET nextrun = (nextrun + $3) WHERE userid= $1 and guildid = $2", uid.id, guild_id, interval,
            )
            .execute(&self.db)
            .await
            {
                Ok(_) => return Ok(()),
                Err(_) => return Err(DatabaseErrors::Error),
            };
        
        }

    pub async fn get_user_id(&self, id: &i64) -> Result<Option<User>, DatabaseErrors> {
        let opt = match sqlx::query!("SELECT * FROM users where id = $1", id)
            .fetch_optional(&self.db)
            .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        match opt {
            Some(e) => {
                return Ok(Some(User {
                    id: e.id.clone(),
                    guild_id: e.guildid.clone(),
                    user_id: e.userid.clone(),
                    praise: e.praise.clone(),
                    praise_name: e.praisename.clone(),
                    timezone: e.timezone.clone(),
                }));
            }
            None => return Ok(None),
        };
    }

    pub async fn get_users_guild(&self, guild_id: &i64) -> Result<Vec<User>, DatabaseErrors> {
        let list = match sqlx::query!("SELECT * FROM users where guildid = $1", guild_id)
            .fetch_all(&self.db)
            .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        return Ok(list.iter().map(|e|User {
                    id: e.id.clone(),
                    guild_id: e.guildid.clone(),
                    user_id: e.userid.clone(),
                    praise: e.praise.clone(),
                    praise_name: e.praisename.clone(),
                    timezone: e.timezone.clone(),
                } ).collect());

            }

    pub async fn get_user_guild(
        &self,
        guild_id: &i64,
        user_id: &i64,
    ) -> Result<Option<User>, DatabaseErrors> {
        let opt = match sqlx::query!(
            "SELECT * FROM users where guildid = $1 AND userid = $2",
            guild_id,
            user_id
        )
        .fetch_optional(&self.db)
        .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        match opt {
            Some(e) => {
                return Ok(Some(User {
                    id: e.id.clone(),
                    guild_id: e.guildid.clone(),
                    user_id: e.userid.clone(),
                    praise: e.praise.clone(),
                    praise_name: e.praisename.clone(),
                    timezone: e.timezone.clone(),
                }));
            }
            None => return Ok(None),
        };
    }

    pub async fn update_user(
        &self,
        user: &User,
    ) -> Result<User, DatabaseErrors> {
        let opt = match self.get_user_guild(&user.guild_id, &user.user_id).await {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match opt {
            Some(e) => {
                match sqlx::query!(
                    "UPDATE users SET (praise, praisename, timezone) = ($1, $2, $3) WHERE id = $4",
                    user.praise,
                    user.praise_name,
                    user.timezone,
                    e.id
                )
                .execute(&self.db)
                .await
                {
                    Ok(_) => return Ok(user.clone()),
                    Err(_) => return Err(DatabaseErrors::Error),
                };
            }
            None => return Err(DatabaseErrors::UserDoesNotExist),
        };
    }

    pub async fn delete_user(&self, guild_id: &i64, user_id: &i64) -> Result<(), DatabaseErrors> {
        let opt = match self.get_user_guild(guild_id, user_id).await {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match opt {
            Some(e) => {
                match sqlx::query!("DELETE FROM schedule WHERE guildid = $1 AND userid = $2", guild_id, e.id)
                    .execute(&self.db)
                    .await
                {
                    Ok(_) => (),
                    Err(_) => return Err(DatabaseErrors::Error),
                };

                match sqlx::query!("DELETE FROM users WHERE id = $1", e.id)
                    .execute(&self.db)
                    .await
                {
                    Ok(_) => return Ok(()),
                    Err(_) => return Err(DatabaseErrors::Error),
                };
            }
            None => return Err(DatabaseErrors::UserDoesNotExist),
        };
    }

    pub async fn add_user(&self, user: &User) -> Result<User, DatabaseErrors> {
        let opt = match self.get_guild(&user.guild_id).await {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match opt {
            Some(e) => {
                let u = match self.get_user_guild(&user.guild_id, &user.user_id).await {
                    Ok(u) => u,
                    Err(u) => return Err(u)
                };

                if u.is_some() {
                    return Err(DatabaseErrors::UserAlreadyExists);
                }

                match sqlx::query!(
                    "INSERT INTO users (guildid, userid, praise, praisename, timezone) VALUES ($1, $2, $3, $4, $5) RETURNING id",
                    e.id,
                    user.user_id,
                    user.praise,
                    user.praise_name,
                    user.timezone,
                )
                .fetch_one(&self.db)
                .await
                {
                    Ok(u) => return Ok(User{
                        id: u.id.clone(),
                        guild_id: e.id.clone(),
                        user_id: user.user_id.clone(),
                        praise_name: user.praise_name.clone(),
                        praise: user.praise.clone(),
                        timezone: user.timezone.clone()
                        
                    }),
                    Err(_) => return Err(DatabaseErrors::Error),
                };
            }
            None => return Err(DatabaseErrors::GuildDoesNotExist),
        };
    }
}
