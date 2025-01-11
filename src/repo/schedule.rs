use std::collections::HashMap;
use std::sync::Arc;

use tokio::spawn;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use chrono;

use sqlx::types::chrono::{DateTime, Utc};

use poise::serenity_prelude as serenity;

use crate::repo::database::*;
use crate::{Context, Data, Error};

#[derive(Clone)]
pub struct Message {
    message: serenity::MessageId,
    datetime: DateTime<Utc>,
    guild: i64,
    channel: i64,
    schedule: Schedule,
}

pub struct Scheduler {
    db: Database,
    active_messages: Arc<Mutex<HashMap<u64, Message>>>,
    message_map: Arc<Mutex<HashMap<i64, u64>>>,
    http: Arc<serenity::http::Http>,
}

impl Scheduler {
    pub fn start(
        db: Database,
        http: Arc<serenity::http::Http>,
        tailwag: String,
        active_messages: Arc<Mutex<HashMap<u64, Message>>>,
    ) {
        let db_clone = db.clone();
        let http_clone = http.clone();
        let tailwag_clone = tailwag.clone();
        let active_messages_clone = active_messages.clone();

        let message_map: Arc<Mutex<HashMap<i64, u64>>> = Arc::new(Mutex::new(HashMap::new()));

        let message_map_clone = message_map.clone();

        tokio::spawn(async move {
            loop {
                Scheduler::process_schedule(
                    &db_clone,
                    &active_messages_clone,
                    &message_map_clone,
                    &http_clone,
                    &tailwag_clone,
                )
                .await;
                sleep(Duration::from_secs(60)).await;
            }
        });

        tokio::spawn(async move {
            loop {
                Scheduler::check_messages(&active_messages, &message_map, &http).await;
                sleep(Duration::from_secs(60)).await;
            }
        });
    }

    async fn process_schedule(
        db: &Database,
        active_messages: &Arc<Mutex<HashMap<u64, Message>>>,
        message_map: &Arc<Mutex<HashMap<i64, u64>>>,
        http: &Arc<serenity::http::Http>,
        tailwag: &String,
    ) {
        let schedules: Vec<Schedule> = match db.get_task_nextrun(None).await {
            Ok(e) => e,
            Err(_) => {
                println!("Cannot fetch schedule due to database error");
                return;
            }
        };

        for schedule in schedules {
            let guild = serenity::GuildId::from(schedule.guild_id as u64);

            let channel = match guild.channels(&http).await {
                Ok(e) => match e.get(&serenity::ChannelId::from(schedule.channel_id as u64)) {
                    Some(c) => c.clone(),
                    None => {
                        println!(
                            "Cannot find channel {} in guild {}",
                            schedule.channel_id, schedule.guild_id
                        );
                        continue;
                    }
                },
                Err(_) => {
                    println!("Cannot fetch channels from guild {}", schedule.guild_id);
                    continue;
                }
            };

            let user = serenity::UserId::from(schedule.user_id as u64);

            let message = serenity::MessageBuilder::new()
                .push("Reminder pup paws at you ")
                .mention(&user)
                .push(format!("{}\n", tailwag))
                .push("It's time for you to ")
                .push_bold(format!("{}\n", schedule.task))
                .push(format!(
                    "Please react once you've {}",
                    schedule.task_secondary
                ))
                .build();

            match channel.say(&http, &message).await {
                Ok(e) => {
                    let mut message_map_lock = message_map.lock().await;
                    let mut message_lock = active_messages.lock().await;

                    match message_map_lock.get(&schedule.id) {
                        Some(e) => {
                            message_lock.remove(e);
                        }
                        None => (),
                    };

                    let message_id: serenity::MessageId = e.into();

                    message_map_lock.insert(schedule.id.clone(), message_id.get());

                    message_lock.insert(
                        message_id.get(),
                        Message {
                            message: message_id,
                            datetime: Utc::now(),
                            guild: schedule.guild_id.clone(),
                            channel: schedule.channel_id.clone(),
                            schedule: schedule.clone(),
                        },
                    );

                    match db.incriment_task(&schedule.id).await {
                        Ok(e) => continue,
                        Err(_) => {
                            println!("Cannot incriment task {}", schedule.id);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "Cannot send message to channel {} in guild {}",
                        schedule.channel_id, schedule.guild_id
                    );
                    println!("{:?}", e);
                    continue;
                }
            }
        }
    }
    async fn check_messages(
        active_messages: &Arc<Mutex<HashMap<u64, Message>>>,
        message_map: &Arc<Mutex<HashMap<i64, u64>>>,
        http: &serenity::http::Http,
    ) {
        let mut messages = active_messages.lock().await;
        let mut messages_to_remove: Vec<u64> = Vec::new();
        let mut messages_to_add: Vec<Message> = Vec::new();
        let mut message_map_lock = message_map.lock().await;
        let messages_itter = &mut *messages;

        let now = Utc::now();

        for (k, v) in messages_itter.iter() {
            if (v.datetime + chrono::Duration::minutes(60)) < now {
                let guild = serenity::GuildId::from(v.guild as u64);

                let channel = match guild.channels(&http).await {
                    Ok(e) => match e.get(&serenity::ChannelId::from(v.channel as u64)) {
                        Some(c) => c.clone(),
                        None => {
                            println!("Cannot find channel {} in guild {}", v.channel, v.guild);
                            continue;
                        }
                    },
                    Err(_) => {
                        println!("Cannot fetch channels from guild {}", v.guild);
                        continue;
                    }
                };

                let message = match channel.message(http, v.message).await {
                    Ok(e) => e,
                    Err(e) => {
                        println!(
                            "Cannot get message {} from channel {} in guild {}",
                            v.message.get(),
                            v.channel,
                            v.guild
                        );
                        println!("{}", e);
                        continue;
                    }
                };

                messages_to_remove.push(k.clone());

                if message.reactions.len() >= 1 {
                    continue;
                }

                let user = serenity::UserId::from(v.schedule.user_id as u64);

                let message = serenity::MessageBuilder::new()
                    .mention(&user)
                    .push("it's been an hour and you havn't")
                    .push_bold(format!("{}\n", v.schedule.task_secondary))
                    .push(format!("This makes puppy sad\n please {}", v.schedule.task,))
                    .build();

                match channel.say(&http, &message).await {
                    Ok(e) => {
                        let message_id: serenity::MessageId = e.into();

                        message_map_lock.insert(v.schedule.id.clone(), message_id.get());

                        messages_to_add.push(Message {
                            message: message_id,
                            datetime: Utc::now(),
                            guild: v.guild.clone(),
                            channel: v.channel.clone(),
                            schedule: v.schedule.clone(),
                        });
                    }
                    Err(e) => {
                        println!(
                            "Cannot send message to channel {} in guild {}",
                            v.channel, v.guild
                        );
                        println!("{:?}", e);
                        continue;
                    }
                }
            }
        }

        for v in messages_to_add {
            messages.insert(v.message.get(), v.clone());
        }

        for k in messages_to_remove {
            messages.remove(&k);
        }
    }
}

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    println!(
        "Got an event in event handler: {:?}",
        event.snake_case_name()
    );

    match event {
        serenity::FullEvent::ReactionAdd { add_reaction } => {
            if add_reaction.message_author_id.unwrap()
                != ctx
                    .http
                    .get_current_application_info()
                    .await
                    .expect("Can't get current appilacion info")
                    .id
                    .get()
            {
                return Ok(());
            }

            let mut messages = data.active_messages.lock().await;

            let m = messages.get(&add_reaction.message_id.get());

            match m {
                Some(e) => {
                    let reponse = serenity::MessageBuilder::new()
                        .push("YAY ")
                        .mention(&add_reaction.user_id.unwrap())
                        .push(format!(" you've {}!!\n", e.schedule.task_secondary))
                        .push(format!(
                            "You've been such a {} I'll give you {}!!!",
                            e.schedule.praise_name, e.schedule.praise
                        ))
                        .build();

                    match add_reaction
                        .message(&ctx.http)
                        .await
                        .unwrap()
                        .reply(&ctx.http, reponse)
                        .await
                    {
                        Ok(_) => (),
                        Err(e) => {
                            println!(
                                "Cannot reply to message {} from channel {} in guild {}",
                                add_reaction.message_id.get(),
                                add_reaction.channel_id.get(),
                                match add_reaction.guild_id {
                                    Some(e) => e.get().to_string(),
                                    None => "NOID".to_string(),
                                }
                            );
                            println!("{}", e);
                        }
                    };
                    messages.remove(&add_reaction.message_id.get());
                }
                None => {
                    let reponse = serenity::MessageBuilder::new()
                        .push("Puppy's memory can only rember the latest reminder")
                        .mention(&add_reaction.user_id.unwrap())
                        .push("\n please react to the latest reminder so puppy can remember it")
                        .build();

                    match add_reaction
                        .message(&ctx.http)
                        .await
                        .unwrap()
                        .reply(&ctx.http, reponse)
                        .await
                    {
                        Ok(_) => (),
                        Err(e) => {
                            println!(
                                "Cannot reply to message {} from channel {} in guild {}",
                                add_reaction.message_id.get(),
                                add_reaction.channel_id.get(),
                                match add_reaction.guild_id {
                                    Some(e) => e.get().to_string(),
                                    None => "NOID".to_string(),
                                }
                            );
                            println!("{}", e);
                        }
                    };
                }
            }
        }
        _ => {}
    }
    Ok(())
}
