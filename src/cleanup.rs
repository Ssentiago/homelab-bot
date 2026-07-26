#![allow(dead_code)]

use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId};

/// Soft-fail: warn on error, never propagate.
pub async fn delete_message(bot: &Bot, chat_id: i64, message_id: i64) {
    let Ok(mid) = i32::try_from(message_id) else {
        tracing::warn!("deleteMessage: invalid message_id {}", message_id);
        return;
    };
    if let Err(e) = bot.delete_message(ChatId(chat_id), MessageId(mid)).await {
        tracing::warn!("deleteMessage failed for {}: {}", mid, e);
    }
}

/// Delete multiple messages, soft-fail each.
pub async fn batch_delete(bot: &Bot, chat_id: i64, ids: &[i64]) {
    for &id in ids {
        delete_message(bot, chat_id, id).await;
    }
}
