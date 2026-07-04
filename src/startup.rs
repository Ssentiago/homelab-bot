use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, ThreadId};

use crate::config::Config;

pub async fn ensure_topics_exist(bot: &Bot, config: &mut Config) -> anyhow::Result<()> {
    if !config.topics_not_created() {
        send_welcome_messages(bot, config).await?;
        return Ok(());
    }

    println!("Creating forum topics...");

    let notifications_topic = bot
        .create_forum_topic(ChatId(config.chat_id), "Уведомления")
        .await?;

    let quick_notes_topic = bot
        .create_forum_topic(ChatId(config.chat_id), "Быстрые заметки")
        .await?;

    let notifications_id = notifications_topic.thread_id.0 .0;
    let quick_notes_id = quick_notes_topic.thread_id.0 .0;

    println!("Created topics: notifications={}, quick_notes={}",
        notifications_id, quick_notes_id
    );

    config.save_thread_ids(notifications_id, quick_notes_id);
    println!("Thread IDs saved to .env");

    send_welcome_messages(bot, config).await?;

    Ok(())
}

async fn send_welcome_messages(bot: &Bot, config: &Config) -> anyhow::Result<()> {
    let chat_id = ChatId(config.chat_id);

    if let Some(id) = config.notifications_thread_id {
        bot.send_message(chat_id, "Топик готов. Сюда будут приходить уведомления.")
            .message_thread_id(ThreadId(MessageId(id)))
            .await?;
    }

    if let Some(id) = config.quick_notes_thread_id {
        bot.send_message(chat_id, "Топик готов. Пишите сюда быстрые заметки — бот сохранит.")
            .message_thread_id(ThreadId(MessageId(id)))
            .await?;
    }

    Ok(())
}
