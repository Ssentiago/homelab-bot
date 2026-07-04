mod config;

use std::sync::Arc;
use teloxide::prelude::*;

use config::Config;

pub type BotState = Arc<Config>;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = Arc::new(Config::from_env());
    let _bot = Bot::new(&config.bot_token);

    println!("Bot started");
    println!("Chat ID: {}", config.chat_id);
    println!("Notifications thread: {:?}", config.notifications_thread_id);
    println!("Quick notes thread: {:?}", config.quick_notes_thread_id);
}
