use std::sync::Arc;
use std::time::Duration;

use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, ThreadId};
use tracing::{info, error};

use crate::config::Config;
use crate::queue::TaskQueue;

pub fn spawn_notification_worker(queue: TaskQueue, bot: Bot, _config: Arc<Config>) {
    tokio::spawn(async move {
        info!("Notification worker started");
        loop {
            match queue.claim_next().await {
                Ok(Some(row)) => {
                    let id: i64 = row.get("id");
                    if let Err(e) = process_task(&bot, &row).await {
                        error!("Task {} failed: {}", id, e);
                        let _ = queue.mark_pending_retry(id, &e.to_string()).await;
                    } else {
                        info!("Task {} completed", id);
                        let _ = queue.mark_done(id).await;
                    }
                }
                Ok(None) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    error!("Claim failed: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });
}

async fn process_task(bot: &Bot, row: &SqliteRow) -> Result<(), String> {
    let chat_id: i64 = row.get("chat_id");
    let thread_id: Option<i32> = row.get("thread_id");
    let text: Option<String> = row.get("text");

    let thread_id = thread_id.map(|id| ThreadId(MessageId(id)));

    bot.send_message(ChatId(chat_id), text.as_deref().unwrap_or(""))
        .message_thread_id(thread_id.unwrap_or(ThreadId(MessageId(0))))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
