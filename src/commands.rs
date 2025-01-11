use crate::repo::database::*;
use crate::{Context, Error};

use poise::serenity_prelude as serenity;

use chrono::TimeDelta;
use sqlx::postgres::types::PgInterval;
use sqlx::types::chrono::{DateTime, Utc};

fn generate_task_table(tasks: &Vec<UserTask>) -> String {
    let mut res = serenity::MessageBuilder::new();
    res.push("Here is everything puppy can remember!!!\n");
    res.push(" ID | User | Task | Task Postense | Interval | Next Run with user's datetime | Next Run | Created \n");

    for task in tasks {
        res.push(format!(" {} | ", task.id,));
        res.mention(&serenity::UserId::new(task.user_id as u64));
        res.push(format!(
            " | {} | {} | {} | {} | <t:{}:R> | <t:{}:R>\n",
            task.task,
            task.task_secondary,
            crate::util::pginterval_to_string(&task.interval),
            chrono::DateTime::<chrono::FixedOffset>::from_naive_utc_and_offset(
                task.next_run.naive_utc(),
                chrono::FixedOffset::east_opt(
                    chrono::TimeDelta::microseconds(task.timezone.microseconds).num_seconds()
                        as i32
                )
                .expect("Cannot conver time")
            )
            .to_rfc2822(),
            task.next_run.timestamp(),
            task.created.timestamp()
        ));
    }

    res.build()
}

#[poise::command(
    prefix_command,
    slash_command,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn setchannel(ctx: Context<'_>) -> Result<(), Error> {
    let guild = Guild {
        id: ctx.guild().unwrap().id.get().try_into().unwrap(),
        channel: ctx.channel_id().get().try_into().unwrap(),
    };
    match ctx.data().db.update_guild(&guild).await {
        Ok(_) => {
            let response = format!("Bark Bark!!! You've successfully shown me where my home is!!\nPlease make sure I have permissions to message in this channel");
            ctx.say(response).await?;
            Ok(())
        }
        Err(_) => {
            let response = format!("Server Error");
            ctx.say(response).await?;
            Ok(())
        }
    }
}

#[poise::command(prefix_command, slash_command)]
pub async fn adduser(
    ctx: Context<'_>,
    #[description = "Only admins can specify other users"] user: Option<serenity::User>,
    #[description = "Praise"] praise: String,
    #[description = "Praise name"] praisename: String,
    #[description = "Timezone Hour -12 to 14 allowed"]
    #[min = -12_i8]
    #[max = 14_i8]
    timezonehour: i8,
    #[description = "Timezone minuets 0 to 59 allowed"]
    #[min = 0_u8]
    #[max = 59_u8]
    timezoneminutes: i8,
) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get();

    let delta = match timezonehour >= 0_i8 {
        true => TimeDelta::minutes(timezoneminutes as i64) + TimeDelta::hours(timezonehour as i64),
        false => {
            TimeDelta::minutes((timezoneminutes - (timezoneminutes * 2)) as i64)
                + TimeDelta::hours(timezonehour as i64)
        }
    };
    let interval = PgInterval::try_from(delta).expect("Delta can't convert to interval");

    let user_id = match user {
        Some(e) => e.id,
        None => ctx.author().id,
    };

    let user_data = User {
        id: 1,
        guild_id: guild as i64,
        user_id: user_id.get() as i64,
        praise: praise.clone(),
        praise_name: praisename.clone(),
        timezone: interval,
    };

    match ctx.data().db.add_user(&user_data).await {
        Ok(_) => {
            let response = serenity::MessageBuilder::new()
                .push("Yay I have a new friend!!!\n")
                .mention(&user_id)
                .push(format!(" has been added and they are a {}", praisename))
                .build();
            ctx.say(response).await?;
            return Ok(());
        }
        Err(e) => {
            println!("{:?}", e);
            match e {
                DatabaseErrors::GuildDoesNotExist => {
                    let response = format!("Bark Bark!!!\nI'm new here, please have an admin run /setchannel before adding people.");
                    ctx.say(response).await?;
                    return Ok(());
                }
                DatabaseErrors::UserAlreadyExists => {
                    let response = serenity::MessageBuilder::new()
                        .mention(&user_id)
                        .push(" is already my friend!!\nPlease use /updateuser to update them!!")
                        .build();
                    ctx.say(response).await?;
                    return Ok(());
                }
                _ => return Err("Server Error".into()),
            }
        }
    };
}

#[poise::command(prefix_command, slash_command)]
pub async fn updateuser(
    ctx: Context<'_>,
    #[description = "Only admins can specify other users"] user: Option<serenity::User>,
    #[description = "Praise"] praise: Option<String>,
    #[description = "Praise name"] praisename: Option<String>,
    #[description = "Timezone Hour -12 to 14 allowed"]
    #[min = -12_i8]
    #[max = 14_i8]
    timezonehour: Option<i8>,
    #[description = "Timezone minuets 0 to 59 allowed"]
    #[min = 0_u8]
    #[max = 59_u8]
    timezoneminutes: Option<i8>,
) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get();
    let user_id = match user {
        Some(e) => e.id,
        None => ctx.author().id,
    };

    let user_data_old_opt = match ctx
        .data()
        .db
        .get_user_guild(&(guild as i64), &(user_id.get() as i64))
        .await
    {
        Ok(e) => e,
        Err(e) => match e {
            DatabaseErrors::GuildDoesNotExist => {
                let response = format!("Bark Bark!!!\nI'm new here, please have an admin run /setchannel before adding people.");
                ctx.say(response).await?;
                return Ok(());
            }
            DatabaseErrors::UserDoesNotExist => {
                let response = serenity::MessageBuilder::new()
                    .mention(&user_id)
                    .push(" is not my friend yet\nPlease use /adduser to make them my friend!!")
                    .build();
                ctx.say(response).await?;
                return Ok(());
            }
            _ => return Err("Server Error".into()),
        },
    };

    let user_data_old = match user_data_old_opt {
        Some(e) => e,
        None => {
            let response = serenity::MessageBuilder::new()
                .mention(&user_id)
                .push(" is not my friend yet\nPlease use /adduser to make them my friend!!")
                .build();
            ctx.say(response).await?;
            return Ok(());
        }
    };

    let interval: PgInterval = match timezonehour {
        Some(h) => match timezoneminutes {
            Some(m) => {
                let delta = match h >= 0_i8 {
                    true => TimeDelta::minutes(m as i64) + TimeDelta::hours(h as i64),
                    false => TimeDelta::minutes((m - (m * 2)) as i64) + TimeDelta::hours(h as i64),
                };
                PgInterval::try_from(delta).expect("Delta can't convert to interval")
            }
            None => {
                let response =
                    "Error: if you are updating the timezone both hour and minutes need to be set";
                ctx.say(response).await?;
                return Ok(());
            }
        },
        None => match timezoneminutes {
            Some(_) => {
                let response =
                    "Error: if you are updating the timezone both hour and minutes need to be set";
                ctx.say(response).await?;
                return Ok(());
            }
            None => user_data_old.timezone.clone(),
        },
    };

    let true_praise = match praise {
        Some(e) => e,
        None => user_data_old.praise.clone(),
    };

    let true_praise_name = match praisename {
        Some(e) => e,
        None => user_data_old.praise_name.clone(),
    };

    let user_data = User {
        id: 1,
        guild_id: guild as i64,
        user_id: user_id.get() as i64,
        praise: true_praise,
        praise_name: true_praise_name.clone(),
        timezone: interval,
    };
    println!("testing");
    match ctx.data().db.update_user(&user_data).await {
        Ok(_) => {
            let response = serenity::MessageBuilder::new()
                .push("Yay friend my friend has been updated!!!\n")
                .mention(&user_id)
                .push(format!(" are a {} and my friend!!!", true_praise_name))
                .build();
            ctx.say(response).await?;
            return Ok(());
        }
        Err(e) => return Err("Server Err".into()),
    };
}

#[poise::command(
    prefix_command,
    slash_command,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn deleteuser(ctx: Context<'_>, user: serenity::User) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().id.get() as i64;

    match ctx
        .data()
        .db
        .delete_user(&guild, &(user.id.get() as i64))
        .await
    {
        Ok(_) => {
            let response = serenity::MessageBuilder::new()
                .push("Puppy is sad that friend has to go home.\n")
                .mention(&user.id)
                .push(" has been removed.")
                .build();
            ctx.say(response).await?;
            Ok(())
        }
        Err(e) => match e {
            DatabaseErrors::UserDoesNotExist => {
                let response = serenity::MessageBuilder::new()
                    .mention(&user.id)
                    .push(" has alreday gone home.\n")
                    .build();
                ctx.say(response).await?;
                Ok(())
            }
            _ => {
                println!("{:?}", e);
                Err("Server Error".into())
            }
        },
    }
}

#[poise::command(prefix_command, slash_command)]
pub async fn addschedule(
    ctx: Context<'_>,
    #[description = "Pretence of task"] pretencetask: String,
    #[description = "Postence of task"] postencetask: String,
    #[description = "Hour to start tark, will be offset with users timezone"]
    #[min = 0_i8]
    #[max = 23_i8]
    starthour: i8,
    #[description = "Minuets start tark, will be offset with users timezone"]
    #[min = 0_u8]
    #[max = 59_u8]
    startminuets: i8,
    #[description = "Add days to interval"]
    #[min = 1_u8]
    #[max = 120_u8]
    intervalday: Option<i64>,
    #[description = "Add hours to interval"]
    #[min = 1_u8]
    #[max = 120_u8]
    intervalhour: Option<i64>,
    #[description = "Add minuets to interval"]
    #[min = 1_u8]
    #[max = 120_u8]
    intervalminuets: Option<i64>,
) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get();
    let user_id = ctx.author().id;

    let user_data_opt = match ctx
        .data()
        .db
        .get_user_guild(&(guild as i64), &(user_id.get() as i64))
        .await
    {
        Ok(e) => e,
        Err(e) => match e {
            DatabaseErrors::GuildDoesNotExist => {
                let response = format!("Bark Bark!!!\nI'm new here, please have an admin run /setchannel before adding people.");
                ctx.say(response).await?;
                return Ok(());
            }
            DatabaseErrors::UserDoesNotExist => {
                let response = serenity::MessageBuilder::new()
                    .mention(&user_id)
                    .push(" is not my friend yet\nPlease use /adduser to make them my friend!!")
                    .build();
                ctx.say(response).await?;
                return Ok(());
            }
            _ => return Err("Server Error".into()),
        },
    };

    let user_data = match user_data_opt {
        Some(e) => e,
        None => {
            let response = serenity::MessageBuilder::new()
                .mention(&user_id)
                .push(" is not my friend yet\nPlease use /adduser to make them my friend!!")
                .build();
            ctx.say(response).await?;
            return Ok(());
        }
    };

    let mut duration = chrono::Duration::zero();

    match intervalday {
        Some(e) => duration = duration + chrono::TimeDelta::days(e),
        None => (),
    };

    match intervalhour {
        Some(e) => duration = duration + chrono::TimeDelta::hours(e),
        None => (),
    };
    match intervalminuets {
        Some(e) => duration = duration + chrono::TimeDelta::minutes(e),
        None => (),
    };

    if duration < chrono::TimeDelta::hours(4) {
        let res = "Puppy can only bark every 4 hours.\nPlease set the interval to atleast 4 hours.";
        ctx.say(res).await?;
        return Ok(());
    }

    let duration = PgInterval::try_from(duration).expect("Cannot conver delta into pginterval");

    let now = Utc::now();

    let time_string = format!(
        "{} {}:{}:00 +0000",
        now.date_naive().to_string(),
        starthour,
        startminuets
    );

    let mut datetime =
        DateTime::parse_from_str(&time_string, "%F %T %z").expect("Cannot parse datetime");

    datetime = datetime - TimeDelta::microseconds(user_data.timezone.microseconds);

    if datetime < now {
        datetime = datetime + TimeDelta::days(1);
    }

    let user_delta = TimeDelta::microseconds(user_data.timezone.microseconds);

    match ctx
        .data()
        .db
        .add_task(Task {
            id: 0,
            guild_id: guild as i64,
            user_id: user_data.id,
            task: pretencetask,
            task_secondary: postencetask,
            created: now,
            interval: duration,
            next_run: datetime.to_utc(),
        })
        .await
    {
        Ok(e) => {
            let response = serenity::MessageBuilder::new()
                .push("Puppy will remember a new task for ")
                .mention(&user_id)
                .push(format!(
                    "\nPuppy will remind them to {} starting from {} {} every {}",
                    e.task,
                    (e.next_run + user_delta).naive_utc(),
                    crate::util::format_timezone(&user_data.timezone),
                    crate::util::pginterval_to_string(&duration)
                ))
                .build();
            ctx.say(response).await?;
            return Ok(());
        }
        Err(_) => {
            return Err("Database error".into());
        }
    };
}

#[poise::command(
    prefix_command,
    slash_command,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn addscheduleadmin(
    ctx: Context<'_>,
    #[description = "Pretence of task"] pretencetask: String,
    #[description = "Postence of task"] postencetask: String,
    #[description = "Hour to start tark, will be offset with users timezone"]
    #[min = 0_i8]
    #[max = 23_i8]
    starthour: i8,
    #[description = "Minuets start tark, will be offset with users timezone"]
    #[min = 0_u8]
    #[max = 59_u8]
    startminuets: i8,
    #[description = "Add days to interval"]
    #[min = 1_u8]
    #[max = 120_u8]
    intervalday: Option<i64>,
    #[description = "Add hours to interval"]
    #[min = 1_u8]
    #[max = 120_u8]
    intervalhour: Option<i64>,
    #[description = "Add minuets to interval"]
    #[min = 1_u8]
    #[max = 120_u8]
    intervalminuets: Option<i64>,
    user: Option<serenity::User>,
) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get();
    let user_id = match user {
        Some(e) => e.id,
        None => ctx.author().id,
    };

    let user_data_opt = match ctx
        .data()
        .db
        .get_user_guild(&(guild as i64), &(user_id.get() as i64))
        .await
    {
        Ok(e) => e,
        Err(e) => match e {
            DatabaseErrors::GuildDoesNotExist => {
                let response = format!("Bark Bark!!!\nI'm new here, please have an admin run /setchannel before adding people.");
                ctx.say(response).await?;
                return Ok(());
            }
            DatabaseErrors::UserDoesNotExist => {
                let response = serenity::MessageBuilder::new()
                    .mention(&user_id)
                    .push(" is not my friend yet\nPlease use /adduser to make them my friend!!")
                    .build();
                ctx.say(response).await?;
                return Ok(());
            }
            _ => return Err("Server Error".into()),
        },
    };

    let user_data = match user_data_opt {
        Some(e) => e,
        None => {
            let response = serenity::MessageBuilder::new()
                .mention(&user_id)
                .push(" is not my friend yet\nPlease use /adduser to make them my friend!!")
                .build();
            ctx.say(response).await?;
            return Ok(());
        }
    };

    let mut duration = chrono::Duration::zero();

    match intervalday {
        Some(e) => duration = duration + chrono::TimeDelta::days(e),
        None => (),
    };

    match intervalhour {
        Some(e) => duration = duration + chrono::TimeDelta::hours(e),
        None => (),
    };
    match intervalminuets {
        Some(e) => duration = duration + chrono::TimeDelta::minutes(e),
        None => (),
    };

    if duration < chrono::TimeDelta::hours(4) {
        let res = "Puppy can only bark every 4 hours.\nPlease set the interval to atleast 4 hours.";
        ctx.say(res).await?;
        return Ok(());
    }

    let duration = PgInterval::try_from(duration).expect("Cannot conver delta into pginterval");

    let now = Utc::now();

    let time_string = format!(
        "{} {}:{}:00 +0000",
        now.date_naive().to_string(),
        starthour,
        startminuets
    );

    let mut datetime =
        DateTime::parse_from_str(&time_string, "%F %T %z").expect("Cannot parse datetime");

    datetime = datetime - TimeDelta::microseconds(user_data.timezone.microseconds);

    if datetime < now {
        datetime = datetime + TimeDelta::days(1);
    }

    let user_delta = TimeDelta::microseconds(user_data.timezone.microseconds);

    match ctx
        .data()
        .db
        .add_task(Task {
            id: 0,
            guild_id: guild as i64,
            user_id: user_data.id,
            task: pretencetask,
            task_secondary: postencetask,
            created: now,
            interval: duration,
            next_run: datetime.to_utc(),
        })
        .await
    {
        Ok(e) => {
            let response = serenity::MessageBuilder::new()
                .push("Puppy will remember a new task for ")
                .mention(&user_id)
                .push(format!(
                    "\nPuppy will remind them to {} starting from {} {} every {}",
                    e.task,
                    (e.next_run + user_delta).naive_utc(),
                    crate::util::format_timezone(&user_data.timezone),
                    crate::util::pginterval_to_string(&duration)
                ))
                .build();
            ctx.say(response).await?;
            return Ok(());
        }
        Err(_) => {
            return Err("Database error".into());
        }
    };
}

#[poise::command(
    prefix_command,
    slash_command,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn getscheduleall(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get() as i64;

    let tasks = match ctx.data().db.get_task_guild(&guild).await {
        Ok(e) => e,
        Err(_) => return Err("Database error".into()),
    };

    let res = generate_task_table(&tasks);

    ctx.say(res).await?;
    return Ok(());
}

#[poise::command(prefix_command, slash_command)]
pub async fn getschedule(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get() as i64;
    let user = ctx.author().id.get() as i64;

    let tasks = match ctx.data().db.get_task_user(&guild, &user).await {
        Ok(e) => e,
        Err(_) => return Err("Database error".into()),
    };

    let res = generate_task_table(&tasks);

    ctx.say(res).await?;
    return Ok(());
}

#[poise::command(
    prefix_command,
    slash_command,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn getscheduleadmin(ctx: Context<'_>, user: Option<serenity::User>) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get() as i64;
    let user_id = match user {
        Some(e) => e.id.get(),
        None => ctx.author().id.get(),
    } as i64;

    let tasks = match ctx.data().db.get_task_user(&guild, &user_id).await {
        Ok(e) => e,
        Err(_) => return Err("Database error".into()),
    };

    let res = generate_task_table(&tasks);

    ctx.say(res).await?;
    return Ok(());
}

#[poise::command(prefix_command, slash_command)]
pub async fn deleteschedule(
    ctx: Context<'_>,
    #[description = "ID from of task /getschedule"] id: u32,
) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get() as i64;
    let user_id = ctx.author().id;

    let task_opt = match ctx.data().db.get_task_id(&(id as i64)).await {
        Ok(e) => e,
        Err(_) => return Err("Database error".into()),
    };

    let task = match task_opt {
        Some(e) => e,
        None => {
            let res = "Puppy doesn't remember this task.\nPlase make a task for puppy to remeber with /addschedule";
            ctx.say(res).await?;
            return Ok(());
        }
    };

    if task.guild_id != guild {
        let res = "Puppy doesn't remember this task.\nPlase make a task for puppy to remeber with /addschedule";
        ctx.say(res).await?;
        return Ok(());
    }

    let user = match ctx.data().db.get_user_id(&task.user_id).await {
        Ok(e) => match e {
            Some(u) => u,
            None => return Err("There is a missing user to a task".into()),
        },
        Err(_) => return Err("Database Error".into()),
    };

    if user_id != (user.user_id as u64) {
        let res = serenity::MessageBuilder::new()
            .push("Hey you don't smell like the owner!!\nPlase  have ")
            .mention(&serenity::UserId::new(user.user_id as u64))
            .push(" delete this task")
            .build();
        ctx.say(res).await?;
        return Ok(());
    }

    match ctx.data().db.delete_task(&(id as i64)).await {
        Ok(_) => {
            let res = "Bark Bark!!!\nPuppy has forgotten the task!!!";
            ctx.say(res).await?;
            return Ok(());
        }
        Err(_) => return Err("Database Error".into()),
    };
}

#[poise::command(
    prefix_command,
    slash_command,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn deletescheduleadmin(
    ctx: Context<'_>,
    #[description = "ID from of task /getschedule"] id: u32,
) -> Result<(), Error> {
    let guild = ctx.guild().expect("not buing used in guild").id.get() as i64;
    let user_id = ctx.author().id;

    let task_opt = match ctx.data().db.get_task_id(&(id as i64)).await {
        Ok(e) => e,
        Err(_) => return Err("Database error".into()),
    };

    let task = match task_opt {
        Some(e) => e,
        None => {
            let res = "Puppy doesn't remember this task.\nPlase make a task for puppy to remeber with /addschedule";
            ctx.say(res).await?;
            return Ok(());
        }
    };

    if task.guild_id != guild {
        let res = "Puppy doesn't remember this task.\nPlase make a task for puppy to remeber with /addschedule";
        ctx.say(res).await?;
        return Ok(());
    }

    match ctx.data().db.delete_task(&(id as i64)).await {
        Ok(_) => {
            let res = "Bark Bark!!!\nPuppy has forgotten the task!!!";
            ctx.say(res).await?;
            return Ok(());
        }
        Err(_) => return Err("Database Error".into()),
    };
}
