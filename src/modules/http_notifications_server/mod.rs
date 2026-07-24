use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use teloxide::prelude::Bot;
use tokio::time::Instant;

use crate::config::Config;
use crate::queue::TaskQueue;

mod server;

pub async fn run(bot: Bot, config: Arc<Config>, queue: TaskQueue, alive: Arc<Mutex<Instant>>, ready: Arc<AtomicBool>) {
    server::start(bot, config, queue, alive, ready).await;
}
