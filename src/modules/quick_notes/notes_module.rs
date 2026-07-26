#![allow(dead_code)]

use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::mpsc;
use tokio::time::Instant;

use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId};
use sqlx::{Row, SqlitePool};

use crate::config::Config;
use crate::module::{Action, ModuleError};
use super::handler;

type ActiveBuffer = Arc<tokio::sync::Mutex<Option<handler::InboxBuffer>>>;

pub struct NotesModule {
    buffer: ActiveBuffer,
    chat_id: i64,
    pool: SqlitePool,
    tx: mpsc::Sender<Message>,
    callback_tx: mpsc::Sender<CallbackQuery>,
    user_msg_ids: Vec<i32>,
}

impl NotesModule {
    pub fn new(
        bot: &Bot,
        config: Arc<Config>,
        pool: SqlitePool,
        alive: Arc<StdMutex<Instant>>,
    ) -> Self {
        let chat_id = config.chat_id;
        let (tx, rx) = mpsc::channel(32);
        let (callback_tx, callback_rx) = mpsc::channel(32);
        let buffer = handler::new_buffer();

        tokio::spawn(handler::run(
            bot.clone(),
            config,
            pool.clone(),
            alive,
            buffer.clone(),
            rx,
            callback_rx,
        ));

        Self { buffer, chat_id, pool, tx, callback_tx, user_msg_ids: Vec::new() }
    }

    /// Forward incoming message to handler::run via mpsc
    pub async fn handle_message(&mut self, msg: Message) -> Result<Action, ModuleError> {
        let msg_id = msg.id.0;
        self.tx.send(msg).await.ok();
        self.user_msg_ids.push(msg_id);
        Ok(Action::Noop)
    }

    /// Forward callback or handle locally
    pub async fn handle_callback(&mut self, q: CallbackQuery) -> Result<Action, ModuleError> {
        match q.data.as_deref() {
            Some("notes:history") => {
                Ok(Action::EditUI {
                    text: "📋 История (пока заглушка)".to_string(),
                    keyboard: notes_keyboard(),
                })
            }
            _ => {
                self.callback_tx.send(q).await.ok();
                Ok(Action::Noop)
            }
        }
    }

    /// Close active window + batch-delete all rich windows + user messages
    pub async fn exit(&mut self, bot: &Bot) {
        // Active rich window
        if let Some(buf) = self.buffer.lock().await.take() {
            buf.close_handle.abort();
            let _ = bot.delete_message(ChatId(self.chat_id), MessageId(buf.feedback_msg_id.0)).await;
        }
        // Closed rich windows (stored by handle_callback / start_countdown)
        let rows = sqlx::query("SELECT message_id FROM pending_cleanup WHERE chat_id = ?")
            .bind(self.chat_id)
            .fetch_all(&self.pool)
            .await;
        if let Ok(rows) = rows {
            for row in &rows {
                let mid: i32 = row.get("message_id");
                let _ = bot.delete_message(ChatId(self.chat_id), MessageId(mid)).await;
            }
        }
        let _ = sqlx::query("DELETE FROM pending_cleanup WHERE chat_id = ?")
            .bind(self.chat_id)
            .execute(&self.pool)
            .await;
        // All user messages from this Notes session
        for &mid in &self.user_msg_ids {
            let _ = bot.delete_message(ChatId(self.chat_id), MessageId(mid)).await;
        }
        self.user_msg_ids.clear();
    }
}

pub fn notes_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📋 История", "notes:history"),
        ],
        vec![
            InlineKeyboardButton::callback("← Назад", "menu:main"),
        ],
    ])
}
