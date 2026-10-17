//! Панель UI: rich-сообщения через frankenstein, inline-фолбэк через teloxide.

pub mod screens;

use frankenstein::client_reqwest::Bot as FrankensteinBot;
use frankenstein::methods::{EditMessageTextParams, SendRichMessageParams};
use frankenstein::AsyncTelegramApi;
use sqlx::{Row, SqlitePool};
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, ThreadId};
use tracing::{info, instrument, warn};

use crate::config::Config;
use screens::{fallback, render, Screen};

pub struct UiPanel {
    bot: Bot,
    api: FrankensteinBot,
    chat_id: i64,
    thread_id: i32,
    message_id: i32,
    pool: SqlitePool,
}

impl UiPanel {
    pub async fn new(bot: &Bot, config: &Config, pool: &SqlitePool) -> Self {
        let mut panel = Self {
            bot: bot.clone(),
            api: FrankensteinBot::new(&config.bot_token),
            chat_id: config.chat_id,
            // топик «Панель»; роутер и так не пускает апдейты из General
            thread_id: config.thread_ids.input.unwrap_or(0),
            message_id: 0,
            pool: pool.clone(),
        };
        panel.recover().await;
        panel
    }

    /// Always send a new UI message — deletes the old one first.
    #[instrument(skip(self))]
    pub async fn force_resend(&mut self, screen: &Screen) {
        if self.message_id != 0 {
            info!("deleting old UI panel, message_id={}", self.message_id);
            let _ = self.bot
                .delete_message(ChatId(self.chat_id), MessageId(self.message_id))
                .await;
        }
        self.send(screen).await;
    }

    /// Edit existing UI message. No-op if content is unchanged; on failure — send new.
    #[instrument(skip(self), fields(screen = ?screen))]
    pub async fn edit(&mut self, screen: &Screen) {
        match self.edit_rich(screen).await {
            Ok(_) => {}
            Err(err) => {
                warn!("rich edit failed, falling back to inline UI: {err}");
                self.send(screen).await;
            }
        }
    }

    /// Send a brand new UI message and store its id in SQLite.
    #[instrument(skip(self), fields(screen = ?screen))]
    pub async fn send(&mut self, screen: &Screen) {
        if let Err(err) = self.send_rich(screen).await {
            warn!("rich send failed, falling back to inline UI: {err}");
            let (text, keyboard) = fallback(screen);
            let msg = self.bot
                .send_message(ChatId(self.chat_id), text)
                .message_thread_id(ThreadId(MessageId(self.thread_id)))
                .reply_markup(keyboard)
                .await;
            if let Ok(msg) = msg {
                info!("sent fallback UI panel, message_id={}", msg.id.0);
                self.persist(msg.id.0).await;
            }
        }
    }

    /// Try to recover existing UI message on startup.
    async fn recover(&mut self) {
        let row = sqlx::query("SELECT message_id FROM persistent_ui WHERE key = 'main'")
            .fetch_optional(&self.pool)
            .await;

        if let Ok(Some(row)) = row {
            self.message_id = row.get("message_id");
            if let Err(err) = self.edit_rich(&Screen::Main).await {
                warn!("recover: rich edit failed, sending new UI: {err}");
                self.send(&Screen::Main).await;
            }
        } else {
            self.send(&Screen::Main).await;
        }
    }

    async fn edit_rich(&mut self, screen: &Screen) -> Result<(), String> {
        if self.message_id == 0 {
            return Err("no UI message_id yet".to_string());
        }
        let params = EditMessageTextParams::builder()
            .chat_id(frankenstein::types::ChatId::Integer(self.chat_id))
            .message_id(self.message_id)
            .rich_message(render(screen))
            // сброс inline-клавиатуры, если панель ранее упала в фолбэк
            .reply_markup(
                frankenstein::types::InlineKeyboardMarkup::builder()
                    .inline_keyboard(Vec::<Vec<frankenstein::types::InlineKeyboardButton>>::new())
                    .build(),
            )
            .build();
        match self.api.edit_message_text(&params).await {
            Ok(_) => Ok(()),
            Err(e) => {
                let err = format!("{e}");
                if err.contains("message is not modified") {
                    // экран уже в нужном состоянии — не сбой
                    Ok(())
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn send_rich(&mut self, screen: &Screen) -> Result<(), String> {
        let params = SendRichMessageParams::builder()
            .chat_id(frankenstein::types::ChatId::Integer(self.chat_id))
            .message_thread_id(self.thread_id)
            .rich_message(render(screen))
            .build();
        match self.api.send_rich_message(&params).await {
            Ok(resp) => {
                info!("sent rich UI panel, message_id={}", resp.result.message_id);
                self.persist(resp.result.message_id).await;
                Ok(())
            }
            Err(e) => Err(format!("{e}")),
        }
    }

    async fn persist(&mut self, message_id: i32) {
        self.message_id = message_id;
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO persistent_ui (key, message_id, chat_id, thread_id) \
             VALUES ('main', ?, ?, ?)",
        )
        .bind(message_id)
        .bind(self.chat_id)
        .bind(self.thread_id)
        .execute(&self.pool)
        .await;
    }
}
