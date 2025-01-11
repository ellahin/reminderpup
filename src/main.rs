#![warn(clippy::str_to_string)]

mod commands;
mod repo;
mod util;
use dotenvy::dotenv;

use poise::serenity_prelude as serenity;

use std::{
    collections::HashMap,
    env,
    env::var,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::repo::database::Database;

struct Data {
    pub db: repo::database::Database,
    pub active_messages: Arc<tokio::sync::Mutex<HashMap<u64, repo::schedule::Message>>>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenv().expect(".env file not found");

    if env::var("DATABASE_URL").is_err() {
        panic!("DATABASE_URL not in environment vars");
    }
    if env::var("DISCORD_TOKEN").is_err() {
        panic!("DATABASE_URL not in environment vars");
    }
    if env::var("DISCORD_PERMISSION").is_err() {
        panic!("DATABASE_URL not in environment vars");
    }

    let db = Database::new(env::var("DATABASE_URL").unwrap())
        .await
        .expect("Cannot start database");

    let active_messages: Arc<tokio::sync::Mutex<HashMap<u64, repo::schedule::Message>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::setchannel(),
            commands::adduser(),
            commands::deleteuser(),
            commands::updateuser(),
            commands::addschedule(),
            commands::addscheduleadmin(),
            commands::getscheduleall(),
            commands::getschedule(),
            commands::getscheduleadmin(),
            commands::deleteschedule(),
            commands::deletescheduleadmin(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            additional_prefixes: vec![
                poise::Prefix::Literal("hey bot,"),
                poise::Prefix::Literal("hey bot"),
            ],
            ..Default::default()
        },
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        // This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        // This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        // Every command invocation must pass this check to continue execution
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 123456789 {
                    return Ok(false);
                }
                Ok(true)
            })
        }),
        // Enforce command checks even for owners (enforced by default)
        // Set to true to bypass checks, which is useful for testing
        skip_checks_for_owners: false,
        event_handler: |ctx, event, framework, data| {
            Box::pin(
                async move { repo::schedule::event_handler(ctx, event, framework, data).await },
            )
        },
        ..Default::default()
    };

    let db_clone = db.clone();
    let active_messages_clone = active_messages.clone();

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    db: db_clone,
                    active_messages: active_messages_clone,
                })
            })
        })
        .options(options)
        .build();

    let token = var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` env var, see README for more information.");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .unwrap();

    let http = client.http.clone();

    let tailwag_emoji = var("DISCORD_TAILWAG")
        .expect("Missing `DISCORD_TAILWAG` env var, see README for more information.");

    repo::schedule::Scheduler::start(db, http, tailwag_emoji, active_messages);

    client.start().await.unwrap();
}
