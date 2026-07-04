use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::post,
    Json, Router,
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, ThreadId};
use tracing::{info, error};

use crate::config::Config;

#[derive(Deserialize)]
struct NotifyRequest {
    message: String,
    #[serde(default = "default_level")]
    level: String,
    #[serde(default)]
    source: Option<String>,
}

fn default_level() -> String {
    "info".to_string()
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

struct AppState {
    bot: Bot,
    config: Arc<Config>,
}

pub async fn run(bot: Bot, config: Arc<Config>) {
    info!("HTTP notifications server task started on port {}", config.notify_server_port);

    let state = Arc::new(AppState { bot, config: config.clone() });

    let app = Router::new()
        .route("/notify", post(handle_notify))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.notify_server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("HTTP server listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if token != state.config.notify_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

async fn handle_notify(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NotifyRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if payload.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "message is required and cannot be empty".to_string(),
            }),
        ));
    }

    let bot = state.bot.clone();
    let config = state.config.clone();
    let message = payload.message.clone();
    let level = payload.level.clone();
    let source = payload.source.clone();

    tokio::spawn(async move {
        send_notification_with_retry(bot, config, &message, &level, source.as_deref()).await;
    });

    Ok(StatusCode::OK)
}

async fn send_notification_with_retry(
    bot: Bot,
    config: Arc<Config>,
    message: &str,
    level: &str,
    source: Option<&str>,
) {
    let text = format_notification(message, level, source);
    let chat_id = ChatId(config.chat_id);
    let thread_id = config.thread_ids.notifications.map(|id| ThreadId(MessageId(id)));

    let delays = [0, 1, 5];

    for (attempt, delay) in delays.iter().enumerate() {
        if *delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(*delay)).await;
        }

        let result = bot
            .send_message(chat_id, &text)
            .message_thread_id(thread_id.unwrap_or(ThreadId(MessageId(0))))
            .await;

        match result {
            Ok(_) => {
                info!("Notification sent successfully (attempt {})", attempt + 1);
                return;
            }
            Err(e) => {
                error!("Failed to send notification (attempt {}): {}", attempt + 1, e);
            }
        }
    }

    error!("All retry attempts failed for notification: {}", text);
    log_failed_notification(message, level, source).await;
}

fn format_notification(message: &str, level: &str, source: Option<&str>) -> String {
    match source {
        Some(src) => format!("[{}] {}: {}", level, src, message),
        None => format!("[{}]: {}", level, message),
    }
}

async fn log_failed_notification(message: &str, level: &str, source: Option<&str>) {
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%z");
    let source_str = source.unwrap_or("unknown");
    let log_line = format!("{} | source={} level={} | {}\n", timestamp, source_str, level, message);

    use tokio::io::AsyncWriteExt;
    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("failed_notifications.log")
        .await
    {
        if let Err(e) = file.write_all(log_line.as_bytes()).await {
            error!("Failed to write to failed_notifications.log: {}", e);
        }
    } else {
        error!("Failed to open failed_notifications.log");
    }
}
