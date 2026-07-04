use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Local;
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, ThreadId};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::info;

use crate::config::Config;

const TITLE_MAX_LEN: usize = 50;

pub struct InboxBuffer {
    file_path: PathBuf,
    close_handle: JoinHandle<()>,
    feedback_msg_id: MessageId,
}

pub type ActiveBuffer = Arc<Mutex<Option<InboxBuffer>>>;

pub fn new_buffer() -> ActiveBuffer {
    Arc::new(Mutex::new(None))
}

pub async fn run(bot: Bot, config: Arc<Config>, buffer: ActiveBuffer) {
    info!("Quick notes task started");

    let quick_notes_thread_id = config
        .thread_ids
        .quick_notes
        .map(|id| ThreadId(MessageId(id)));

    let handler = move |bot: Bot, msg: Message| {
        let config = config.clone();
        let buffer = buffer.clone();
        let thread_id = quick_notes_thread_id;
        async move {
            if let Some(thread_id) = thread_id
                && msg.thread_id == Some(thread_id)
            {
                handle_message(bot, msg, config, buffer).await?;
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }
    };

    Dispatcher::builder(bot, Update::filter_message().endpoint(handler))
        .build()
        .dispatch()
        .await;
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    config: Arc<Config>,
    buffer: ActiveBuffer,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };

    let mut buf = buffer.lock().await;

    if let Some(ref mut active) = *buf {
        active.close_handle.abort();

        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&active.file_path)
            .await?;

        use tokio::io::AsyncWriteExt;
        file.write_all(format!("\n\n{}\n", text).as_bytes()).await?;

        let feedback_msg_id = active.feedback_msg_id;
        let filename = active.file_path.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown")
            .to_string();
        let buffer_clone = buffer.clone();
        let debounce_secs = config.debounce_secs;
        let bot_clone = bot.clone();
        let chat_id = ChatId(config.chat_id);

        active.close_handle = tokio::spawn(async move {
            start_countdown(bot_clone, chat_id, feedback_msg_id, &filename, debounce_secs, buffer_clone).await;
        });
    } else {
        let title = truncate_at_word_boundary(first_line(text), TITLE_MAX_LEN);
        let slug = slugify(title);
        let now = Local::now();
        let mut filename = format!("{}_{}.md", now.format("%Y-%m-%d_%H-%M"), slug);

        let root = Path::new(&config.root);

        let mut file_path = root.join(&filename);
        let mut counter = 2;
        while file_path.exists() {
            filename = format!("{}_{}-{}.md", now.format("%Y-%m-%d_%H-%M"), slug, counter);
            file_path = root.join(&filename);
            counter += 1;
        }

        let frontmatter = format!(
            "---\ncreated: {}\nsource: telegram\n---\n\n",
            now.to_rfc3339()
        );

        let content = text.lines().skip(1).collect::<Vec<_>>().join("\n");
        tokio::fs::write(&file_path, format!("{}{}\n", frontmatter, content)).await?;

        let chat_id = ChatId(config.chat_id);
        let thread_id = config.thread_ids.quick_notes.map(|id| ThreadId(MessageId(id)));
        let initial_text = format!("Файл сохранён: {}\n\nОкно: {} сек", filename, config.debounce_secs);

        let feedback_msg = bot
            .send_message(chat_id, &initial_text)
            .message_thread_id(thread_id.unwrap_or(ThreadId(MessageId(0))))
            .await?;

        let feedback_msg_id = feedback_msg.id;
        let buffer_clone = buffer.clone();
        let debounce_secs = config.debounce_secs;
        let bot_clone = bot.clone();
        let filename_clone = filename.clone();

        let close_handle = tokio::spawn(async move {
            start_countdown(bot_clone, chat_id, feedback_msg_id, &filename_clone, debounce_secs, buffer_clone).await;
        });

        *buf = Some(InboxBuffer {
            file_path,
            close_handle,
            feedback_msg_id,
        });
    }

    Ok(())
}

async fn start_countdown(
    bot: Bot,
    chat_id: ChatId,
    msg_id: MessageId,
    filename: &str,
    debounce_secs: u64,
    buffer: ActiveBuffer,
) {
    for remaining in (1..debounce_secs).rev() {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let _ = bot
            .edit_message_text(chat_id, msg_id, format!("Файл сохранён: {}\n\nОкно: {} сек", filename, remaining))
            .await;
    }

    let _ = bot
        .edit_message_text(chat_id, msg_id, format!("Файл сохранён: {}", filename))
        .await;

    let mut buf = buffer.lock().await;
    *buf = None;
}

fn slugify(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| !"/\\:*?\"<>|#".contains(*c))
        .collect();
    let slug = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    slug.trim_matches('-').to_string()
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

fn truncate_at_word_boundary(s: &str, max_len: usize) -> &str {
    if s.chars().count() <= max_len {
        return s;
    }
    let end = s.char_indices()
        .nth(max_len)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let truncated = &s[..end];
    truncated.rfind(' ').map_or(truncated, |i| &truncated[..i])
}
