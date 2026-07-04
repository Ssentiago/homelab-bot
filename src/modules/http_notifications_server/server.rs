use std::sync::Arc;

use axum::{
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::post,
    Json, Router,
};
use chrono::Local;
use serde::Serialize;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile, MessageId, ThreadId};
use tracing::{info, error};

use crate::config::Config;

#[derive(Serialize)]
pub(crate) struct ErrorResponse {
    error: String,
}

struct AppState {
    bot: Bot,
    config: Arc<Config>,
}

pub async fn start(bot: Bot, config: Arc<Config>) {
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
    mut multipart: Multipart,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut message = None;
    let mut level = "info".to_string();
    let mut source = None;
    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e: axum::extract::multipart::MultipartError| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() }))
    })? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "message" => {
                let text = field.text().await.map_err(|e: axum::extract::multipart::MultipartError| {
                    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() }))
                })?;
                message = Some(text);
            }
            "level" => {
                level = field.text().await.map_err(|e: axum::extract::multipart::MultipartError| {
                    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() }))
                })?;
            }
            "source" => {
                source = Some(field.text().await.map_err(|e: axum::extract::multipart::MultipartError| {
                    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() }))
                })?);
            }
            "file" => {
                let filename = field
                    .file_name()
                    .unwrap_or("file.bin")
                    .to_string();
                let data = field.bytes().await.map_err(|e: axum::extract::multipart::MultipartError| {
                    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() }))
                })?;
                file_data = Some((filename, data.to_vec()));
            }
            _ => {}
        }
    }

    let message = message.ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "message is required".to_string(),
        }),
    ))?;

    if message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "message cannot be empty".to_string(),
            }),
        ));
    }

    let bot = state.bot.clone();
    let config = state.config.clone();

    tokio::spawn(async move {
        send_notification_with_retry(bot, config, &message, &level, source.as_deref(), file_data).await;
    });

    Ok(StatusCode::OK)
}

async fn send_notification_with_retry(
    bot: Bot,
    config: Arc<Config>,
    message: &str,
    level: &str,
    source: Option<&str>,
    file_data: Option<(String, Vec<u8>)>,
) {
    let text = format_notification(message, level, source);
    let chat_id = ChatId(config.chat_id);
    let thread_id = config.thread_ids.notifications.map(|id| ThreadId(MessageId(id)));

    let delays = [0, 1, 5];

    for (attempt, delay) in delays.iter().enumerate() {
        if *delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(*delay)).await;
        }

        let result = if let Some((filename, data)) = &file_data {
            let input_file = InputFile::memory(data.clone()).file_name(filename.clone());
            bot.send_document(chat_id, input_file)
                .caption(&text)
                .message_thread_id(thread_id.unwrap_or(ThreadId(MessageId(0))))
                .await
        } else {
            bot.send_message(chat_id, &text)
                .message_thread_id(thread_id.unwrap_or(ThreadId(MessageId(0))))
                .await
        };

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
