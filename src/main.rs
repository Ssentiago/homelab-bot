mod config;
mod modules;
mod startup;

use std::sync::Arc;
use teloxide::prelude::*;

use config::Config;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let mut config = Config::from_env();

    let bot = Bot::new(&config.bot_token);

    println!("Homelab Bot starting...");
    println!("Chat ID: {}", config.chat_id);

    if let Err(e) = startup::ensure_topics_exist(&bot, &mut config).await {
        eprintln!("Failed to create topics: {}", e);
        std::process::exit(1);
    }

    let config = Arc::new(config);
    println!("Notifications thread: {:?}", config.notifications_thread_id);
    println!("Quick notes thread: {:?}", config.quick_notes_thread_id);

    let bot_handle = tokio::spawn(modules::quick_notes::run(bot.clone(), config.clone()));
    let http_handle = tokio::spawn(modules::http_notifications_server::run(bot.clone(), config.clone()));

    tokio::select! {
        res = bot_handle => {
            println!("Bot task exited: {:?}", res);
        }
        res = http_handle => {
            println!("HTTP server task exited: {:?}", res);
        }
    }
}
