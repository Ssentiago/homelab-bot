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
    message_count: u32,
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

    let explicit_title = extract_explicit_title(text);
    let has_marker = explicit_title.is_some();

    let (slug, content) = if let Some(title_raw) = explicit_title {
        let title = truncate_at_word_boundary(title_raw, TITLE_MAX_LEN);
        let slug = slugify(title);
        let content = text.lines().skip(1).collect::<Vec<_>>().join("\n");
        (slug, content)
    } else {
        (String::new(), text.to_string())
    };

    let mut buf = buffer.lock().await;

    if has_marker {
        if let Some(ref mut active) = *buf {
            active.close_handle.abort();
            *buf = None;
        }
    } else {
        if let Some(ref mut active) = *buf {
            active.close_handle.abort();

            let mut file = tokio::fs::OpenOptions::new()
                .append(true)
                .open(&active.file_path)
                .await?;

            use tokio::io::AsyncWriteExt;
            file.write_all(format!("\n\n{}\n", text).as_bytes()).await?;

            active.message_count += 1;
            let message_count = active.message_count;
            let feedback_msg_id = active.feedback_msg_id;
            let filename = active.file_path.file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unknown")
                .to_string();
            let buffer_clone = buffer.clone();
            let debounce_secs = config.debounce_secs;
            let bot_clone = bot.clone();
            let chat_id = ChatId(config.chat_id);

            let append_text = format!("Добавлено в {}\nСообщений: {}\n\nОкно: {} сек", filename, message_count, debounce_secs);
            let _ = bot
                .edit_message_text(chat_id, feedback_msg_id, &append_text)
                .await;

            active.close_handle = tokio::spawn(async move {
                start_countdown(bot_clone, chat_id, feedback_msg_id, &filename, message_count, debounce_secs, buffer_clone).await;
            });

            return Ok(());
        }
    }

    let now = Local::now();
    let filename = if slug.is_empty() {
        format!("{}.md", now.format("%Y-%m-%d_%H-%M"))
    } else {
        format!("{}_{}.md", now.format("%Y-%m-%d_%H-%M"), slug)
    };

    let root = Path::new(&config.root);

    let mut file_path = root.join(&filename);
    let mut counter = 2;
    while file_path.exists() {
        let new_filename = if slug.is_empty() {
            format!("{}-{}.md", now.format("%Y-%m-%d_%H-%M"), counter)
        } else {
            format!("{}_{}-{}.md", now.format("%Y-%m-%d_%H-%M"), slug, counter)
        };
        file_path = root.join(&new_filename);
        counter += 1;
    }

    let frontmatter = format!(
        "---\ncreated: {}\nsource: telegram\n---\n\n",
        now.to_rfc3339()
    );

    tokio::fs::write(&file_path, format!("{}{}\n", frontmatter, content)).await?;

    let chat_id = ChatId(config.chat_id);
    let thread_id = config.thread_ids.quick_notes.map(|id| ThreadId(MessageId(id)));
    let initial_text = format!("Файл сохранён: {}\nСообщений: 1\n\nОкно: {} сек", filename, config.debounce_secs);

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
        start_countdown(bot_clone, chat_id, feedback_msg_id, &filename_clone, 1, debounce_secs, buffer_clone).await;
    });

    *buf = Some(InboxBuffer {
        file_path,
        close_handle,
        feedback_msg_id,
        message_count: 1,
    });

    Ok(())
}

async fn start_countdown(
    bot: Bot,
    chat_id: ChatId,
    msg_id: MessageId,
    filename: &str,
    message_count: u32,
    debounce_secs: u64,
    buffer: ActiveBuffer,
) {
    for remaining in (1..debounce_secs).rev() {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let _ = bot
            .edit_message_text(chat_id, msg_id, format!("Файл сохранён: {}\nСообщений: {}\n\nОкно: {} сек", filename, message_count, remaining))
            .await;
    }

    let _ = bot
        .edit_message_text(chat_id, msg_id, format!("Файл сохранён: {}\nСообщений: {}", filename, message_count))
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

fn extract_explicit_title(text: &str) -> Option<&str> {
    let first = text.lines().next().unwrap_or(text);
    let rest = first.strip_prefix('!')?;
    let trimmed = rest.trim();
    if trimmed.is_empty() { None } else { Some(trimmed) }
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
