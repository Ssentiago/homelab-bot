use std::sync::Arc;
use teloxide::prelude::*;

use crate::config::Config;

pub async fn run(bot: Bot, config: Arc<Config>) {
    println!("HTTP notifications server task started on port {}", config.notify_server_port);
    // TODO: axum server for notifications
}
