use teloxide::prelude::*;
use teloxide::types::ChatId;

use crate::config::Config;

pub async fn ensure_topics_exist(bot: &Bot, config: &mut Config) -> anyhow::Result<()> {
    if !config.topics_not_created() {
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

    Ok(())
}
