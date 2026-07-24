use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, ThreadId};
use tracing::{info, error};

use crate::config::Config;
use crate::queue::{self, PendingNotification};

pub fn spawn_notification_worker(pool: SqlitePool, bot: Bot, _config: Arc<Config>) {
    tokio::spawn(async move {
        info!("Notification worker started");
        loop {
            match queue::claim_next_pending(&pool).await {
                Ok(Some(task)) => {
                    if let Err(e) = process_task(&bot, &task).await {
                        error!("Task {} failed: {}", task.id, e);
                        let _ = queue::mark_pending_retry(&pool, task.id, &e.to_string()).await;
                    } else {
                        info!("Task {} completed", task.id);
                        let _ = queue::mark_done(&pool, task.id).await;
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

async fn process_task(
    bot: &Bot,
    task: &PendingNotification,
) -> Result<(), String> {
    let chat_id = ChatId(task.chat_id);
    let thread_id = task.thread_id.map(|id| ThreadId(MessageId(id)));

    bot.send_message(chat_id, task.text.as_deref().unwrap_or(""))
        .message_thread_id(thread_id.unwrap_or(ThreadId(MessageId(0))))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
