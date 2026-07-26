use teloxide::prelude::*;
use teloxide::types::{BotCommand, ChatId};
use tracing::info;

use crate::config::Config;

pub async fn ensure_topics_exist(bot: &Bot, config: &mut Config) -> anyhow::Result<()> {
    bot.set_my_commands(vec![
        BotCommand::new("start", "Показать главное меню"),
    ]).await?;

    if !config.topics_not_created() {
        return Ok(());
    }

    info!("Creating forum topics...");

    let input_topic = bot
        .create_forum_topic(ChatId(config.chat_id), "Панель")
        .await?;

    let output_topic = bot
        .create_forum_topic(ChatId(config.chat_id), "Мониторинг")
        .await?;

    let input_id = input_topic.thread_id.0 .0;
    let output_id = output_topic.thread_id.0 .0;

    info!("Created topics: input={}, output={}", input_id, output_id);

    config.save_thread_ids(input_id, output_id);
    info!("Thread IDs saved to config.json");

    Ok(())
}
