#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use teloxide::prelude::*;
use tokio::sync::Mutex;
use tracing::info;

use crate::config::Config;
use crate::dedup::DedupCache;
use crate::queue::{self, TaskQueue};

#[derive(Serialize)]
pub(crate) struct ErrorResponse {
    error: String,
}

struct AppState {
    bot: Bot,
    config: Arc<Config>,
    dedup_cache: Mutex<DedupCache>,
    queue: TaskQueue,
}

pub async fn start(bot: Bot, config: Arc<Config>, queue: TaskQueue) {
    info!("HTTP notifications server task started on port {}", config.notify_server_port);

    let state = Arc::new(AppState {
        bot,
        config: config.clone(),
        dedup_cache: Mutex::new(DedupCache::new(Duration::from_secs(300))),
        queue,
    });

    let app = Router::new()
        .route("/notify", post(handle_notify))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route("/", get(handle_help))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.notify_server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("HTTP server listening on {}", addr);
    info!(
        "Send notifications via: curl -X POST http://localhost:{}/notify -H \"Authorization: Bearer <token>\" -F \"message=...\"",
        config.notify_server_port
    );

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

async fn handle_help(State(state): State<Arc<AppState>>) -> String {
    format!(
        "Homelab Bot notify server\n\n\
         POST http://<host>:{port}/notify\n\
         Authorization: Bearer <NOTIFY_TOKEN>\n\
         Content-Type: multipart/form-data\n\n\
         Fields:\n\
         - message (required)\n\
         - level (optional, default: info)\n\
         - source (optional)\n\
         - file (optional, attachment)\n\n\
         Example:\n\
         curl -X POST http://localhost:{port}/notify \\\n\
         \x20\x20-H \"Authorization: Bearer <NOTIFY_TOKEN>\" \\\n\
         \x20\x20-F \"message=Backup completed\" \\\n\
         \x20\x20-F \"level=info\" \\\n\
         \x20\x20-F \"source=backup-script\"\n",
        port = state.config.notify_server_port
    )
}

async fn handle_notify(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut message = None;
    let mut level = "info".to_string();
    let mut source = None;

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
                let _ = field.file_name().unwrap_or("file.bin").to_string();
                let _ = field.bytes().await.map_err(|e: axum::extract::multipart::MultipartError| {
                    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() }))
                })?;
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

    let chat_id = state.config.chat_id;
    let thread_id = state.config.thread_ids.notifications;
    let text = format_notification(&message, &level, source.as_deref());

    let _ = queue::insert_notification(
        state.queue.pool(),
        chat_id,
        thread_id,
        "plain",
        Some(&text),
        None,
        None,
    )
    .await;

    Ok(StatusCode::OK)
}

fn level_emoji(level: &str) -> &str {
    match level {
        "error" => "🔴",
        "warning" => "🟡",
        _ => "🔵",
    }
}

fn format_notification(message: &str, level: &str, source: Option<&str>) -> String {
    let emoji = level_emoji(level);
    match source {
        Some(src) => format!("[{}] {}: {}", emoji, src, message),
        None => format!("[{}]: {}", emoji, message),
    }
}

fn format_notification_with_count(message: &str, level: &str, source: &str, count: u32) -> String {
    let emoji = level_emoji(level);
    format!("[{}] {}: {} ×{}", emoji, source, message, count)
}
