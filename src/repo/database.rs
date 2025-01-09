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
    pub task: String,
    pub task_secondary: String,
    pub praise: String,
    pub praise_name: String,
    pub interval: PgInterval,
    pub created: DateTime<Utc>,
    pub next_run: DateTime<Utc>,
}

#[derive(Debug)]
pub enum DatabaseErrors {
    Error,
    CannotConect,
    MigrationFolderDoesNotExist,
    MigrationError(MigrateError),
    DoesNotExist,
}
#[derive(Clone)]
pub struct Database {
    db: Pool<Postgres>,
}

impl Database {
    pub async fn new(database_url: String) -> Result<Database, DatabaseErrors> {
        let migration_path = Path::new("./migrations");

        let sql_pool = match PgPool::connect(&database_url).await {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::CannotConect),
        };

        let migrator = match Migrator::new(migration_path).await {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::MigrationFolderDoesNotExist),
        };

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

    pub async fn get_task_id(&self, id: &i64) -> Result<Option<Schedule>, DatabaseErrors> {
        let opt = match sqlx::query!("SELECT * FROM schedule WHERE id = $1", id)
            .fetch_optional(&self.db)
            .await
        {
            Ok(e) => e,
            Err(_) => return Err(DatabaseErrors::Error),
        };

        match opt {
            Some(e) => {
                return Ok(Some(Schedule {
                    id: e.id.clone(),
                    guild_id: e.guildid.clone(),
                    user_id: e.userid.clone(),
                    interval: e.interval.clone(),
                    next_run: e.nextrun.clone().and_utc(),
                    created: e.created.clone().and_utc(),
                    praise: e.praise.clone(),
                    praise_name: e.praisename.clone(),
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
    ) -> Result<Vec<Schedule>, DatabaseErrors> {
        let tasks = match sqlx::query!(
            "SELECT * FROM schedule WHERE guildid = $1 and userid = $2",
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
            .map(|e| Schedule {
                id: e.id.clone(),
                guild_id: e.guildid.clone(),
                user_id: e.userid.clone(),
                interval: e.interval.clone(),
                next_run: e.nextrun.clone().and_utc(),
                created: e.created.clone().and_utc(),
                praise: e.praise.clone(),
                praise_name: e.praisename.clone(),
                task: e.task.clone(),
                task_secondary: e.tasksecondary.clone(),
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
            "SELECT * FROM schedule WHERE nextRun < $1",
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
                interval: e.interval.clone(),
                next_run: e.nextrun.clone().and_utc(),
                created: e.created.clone().and_utc(),
                praise: e.praise.clone(),
                praise_name: e.praisename.clone(),
                task: e.task.clone(),
                task_secondary: e.tasksecondary.clone(),
            })
            .collect());
    }

    pub async fn update_task(
        &self,
        id: &i64,
        schedule: Schedule,
    ) -> Result<Schedule, DatabaseErrors> {
        todo!()
    }

    pub async fn delete_task(&self, id: &i64) -> Result<(), DatabaseErrors> {
        todo!()
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
}
