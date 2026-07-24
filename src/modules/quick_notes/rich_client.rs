#![allow(dead_code)]

use std::sync::Arc;

use frankenstein::client_reqwest::Bot;
use frankenstein::methods::{EditMessageTextParams, SendRichMessageParams};
use frankenstein::rich_message::InputRichMessage;
use frankenstein::types::ChatId;
use frankenstein::AsyncTelegramApi;
use tracing::error;

use crate::config::Config;

#[derive(Clone)]
pub struct RichClient {
    api: Bot,
}

impl RichClient {
    pub fn new(config: &Arc<Config>) -> Self {
        let api = Bot::new(&config.bot_token);
        Self { api }
    }

    pub async fn open_window(
        &self,
        chat_id: i64,
        thread_id: Option<i32>,
        remaining_secs: u64,
        content: &str,
    ) -> Result<i32, String> {
        let rich_message = build_rich_message(remaining_secs, content);

        let params = match thread_id {
            Some(tid) => SendRichMessageParams::builder()
                .chat_id(ChatId::Integer(chat_id))
                .message_thread_id(tid)
                .rich_message(rich_message)
                .build(),
            None => SendRichMessageParams::builder()
                .chat_id(ChatId::Integer(chat_id))
                .rich_message(rich_message)
                .build(),
        };

        match self.api.send_rich_message(&params).await {
            Ok(response) => Ok(response.result.message_id),
            Err(e) => {
                error!("Failed to send rich message: {:?}", e);
                Err(format!("send_rich_message failed: {:?}", e))
            }
        }
    }

    pub async fn update_window(
        &self,
        chat_id: i64,
        message_id: i32,
        remaining_secs: Option<u64>,
        filename: Option<&str>,
        content: &str,
    ) -> Result<(), String> {
        let rich_message = match (remaining_secs, filename) {
            (Some(secs), _) => build_rich_message(secs, content),
            (None, Some(name)) => build_final_message(name, content),
            (None, None) => InputRichMessage::builder().markdown(content.to_string()).build(),
        };

        let params = EditMessageTextParams::builder()
            .chat_id(ChatId::Integer(chat_id))
            .message_id(message_id)
            .rich_message(rich_message)
            .build();

        match self.api.edit_message_text(&params).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to edit rich message: {:?}", e);
                Err(format!("edit_message_text failed: {:?}", e))
            }
        }
    }
}

fn build_rich_message(remaining_secs: u64, content: &str) -> InputRichMessage {
    let markdown = format!("# ОКНО: {}\n\n---\n\n{}", remaining_secs, content);
    InputRichMessage::builder()
        .markdown(markdown)
        .build()
}

fn build_final_message(filename: &str, content: &str) -> InputRichMessage {
    let markdown = format!("# Файл сохранён: {}\n\n---\n\n{}", filename, content);
    InputRichMessage::builder()
        .markdown(markdown)
        .build()
}
