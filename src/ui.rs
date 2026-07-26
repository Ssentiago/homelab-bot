#![allow(dead_code)]

use sqlx::{Row, SqlitePool};
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ThreadId};

use crate::config::Config;

pub struct UiPanel {
    bot: Bot,
    chat_id: i64,
    thread_id: i32,
    message_id: i32,
    pool: SqlitePool,
}

impl UiPanel {
    pub async fn new(bot: &Bot, config: &Config, pool: &SqlitePool) -> Self {
        let mut panel = Self {
            bot: bot.clone(),
            chat_id: config.chat_id,
            thread_id: config.thread_ids.input.unwrap_or(0),
            message_id: 0,
            pool: pool.clone(),
        };
        panel.recover().await;
        panel
    }

    /// Always send a new UI message — deletes the old one first.
    pub async fn force_resend(&mut self, text: &str, keyboard: InlineKeyboardMarkup) {
        if self.message_id != 0 {
            let _ = self.bot
                .delete_message(ChatId(self.chat_id), MessageId(self.message_id))
                .await;
        }
        self.send(text, keyboard).await;
    }

    /// Edit existing UI message. If MESSAGE_ID_INVALID — send new, update SQLite.
    /// If the edit is a no-op (same text+keyboard), Telegram returns MESSAGE_NOT_MODIFIED —
    /// silently skip, don't create a duplicate.
    pub async fn edit(&mut self, text: &str, keyboard: InlineKeyboardMarkup) {
        let result = self.bot
            .edit_message_text(ChatId(self.chat_id), MessageId(self.message_id), text)
            .reply_markup(keyboard.clone())
            .await;

        if let Err(ref e) = result {
            let err_str = format!("{e}");
            if err_str.contains("not modified") {
                return;
            }
            self.send(text, keyboard).await;
        }
    }

    /// Send a brand new UI message and store its id in SQLite.
    pub async fn send(&mut self, text: &str, keyboard: InlineKeyboardMarkup) {
        let msg = self.bot
            .send_message(ChatId(self.chat_id), text)
            .message_thread_id(ThreadId(MessageId(self.thread_id)))
            .reply_markup(keyboard)
            .await;

        if let Ok(msg) = msg {
            self.message_id = msg.id.0;
            let _ = sqlx::query(
                "INSERT OR REPLACE INTO persistent_ui (key, message_id, chat_id, thread_id) \
                 VALUES ('main', ?, ?, ?)",
            )
            .bind(self.message_id)
            .bind(self.chat_id)
            .bind(self.thread_id)
            .execute(&self.pool)
            .await;
        }
    }

    /// Try to recover existing UI message on startup.
    async fn recover(&mut self) {
        let version = env!("CARGO_PKG_VERSION");
        let text = format!("Homelab Bot v{}\n\nВыберите действие:", version);
        let keyboard = main_menu_keyboard();

        let row = sqlx::query("SELECT message_id FROM persistent_ui WHERE key = 'main'")
            .fetch_optional(&self.pool)
            .await;

        if let Ok(Some(row)) = row {
            self.message_id = row.get("message_id");
            let result = self.bot
                .edit_message_text(ChatId(self.chat_id), MessageId(self.message_id), &text)
                .reply_markup(keyboard.clone())
                .await;
            if let Err(ref e) = result {
                let err_str = format!("{e}");
                if err_str.contains("not modified") {
                    return;
                }
                self.send(&text, keyboard).await;
            }
        } else {
            self.send(&text, keyboard).await;
        }
    }
}

/// Главное меню: две строки кнопок
pub fn main_menu_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📝 Заметки", "menu:notes"),
            InlineKeyboardButton::callback("🎵 Музыка", "menu:music"),
        ],
        vec![
            InlineKeyboardButton::callback("📊 Саммари", "menu:summary"),
            InlineKeyboardButton::callback("📊 Статус", "menu:status"),
        ],
    ])
}

pub fn back_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("← Назад", "menu:main")],
    ])
}

pub fn main_menu_text() -> String {
    format!("Homelab Bot v{}\n\nВыберите действие:", env!("CARGO_PKG_VERSION"))
}
