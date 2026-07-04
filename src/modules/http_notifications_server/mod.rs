use std::sync::Arc;
use teloxide::prelude::*;
use tracing::info;

use crate::config::Config;

pub async fn run(_bot: Bot, config: Arc<Config>) {
    info!("HTTP notifications server task started on port {}", config.notify_server_port);
    // TODO: axum server for notifications
    std::future::pending::<()>().await;
}
