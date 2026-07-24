use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Local;
use teloxide::prelude::*;
use teloxide::types::MessageId;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::info;

use crate::config::Config;
use super::format;
use super::rich_client::RichClient;

const TITLE_MAX_LEN: usize = 50;

pub struct InboxBuffer {
    file_path: PathBuf,
    close_handle: JoinHandle<()>,
    feedback_msg_id: MessageId,
    message_count: u32,
    message_sequence: u32,
    message_ids: HashMap<MessageId, u32>,
}

pub type ActiveBuffer = Arc<Mutex<Option<InboxBuffer>>>;

pub fn new_buffer() -> ActiveBuffer {
    Arc::new(Mutex::new(None))
}

pub async fn run(
    bot: Bot,
    config: Arc<Config>,
    buffer: ActiveBuffer,
    mut rx: mpsc::Receiver<Message>,
    mut callback_rx: mpsc::Receiver<teloxide::types::CallbackQuery>,
) {
    info!("Quick notes task started");

    let buffer_clone = buffer.clone();
    let config_clone = config.clone();
    let bot_clone = bot.clone();

    tokio::spawn(async move {
        while let Some(query) = callback_rx.recv().await {
            let bot = bot_clone.clone();
            let config = config_clone.clone();
            let buffer = buffer_clone.clone();
            tokio::spawn(async move {
                handle_callback(bot, query, config, buffer).await;
            });
        }
    });

    while let Some(msg) = rx.recv().await {
        let bot = bot.clone();
        let config = config.clone();
        let buffer = buffer.clone();
        let rich_client = RichClient::new(&config);
        tokio::spawn(async move {
            if let Err(e) = handle_message(bot, msg, config, buffer, &rich_client).await {
                tracing::error!("Error handling message: {}", e);
            }
        });
    }
}

async fn handle_message(
    _bot: Bot,
    msg: Message,
    config: Arc<Config>,
    buffer: ActiveBuffer,
    rich_client: &RichClient,
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
    } else if msg.edit_date().is_some() {
        if let Some(ref mut active) = *buf
            && let Some(&seq) = active.message_ids.get(&msg.id)
        {
            active.close_handle.abort();

            let content = tokio::fs::read_to_string(&active.file_path).await?;
            let new_content = format::replace_message(&content, seq, text);
            tokio::fs::write(&active.file_path, &new_content).await?;

            let buffer_clone = buffer.clone();
            let debounce_secs = config.debounce_secs;
            let chat_id = config.chat_id;
            let feedback_msg_id = active.feedback_msg_id;
            let rich_client_clone = rich_client.clone();

            active.close_handle = tokio::spawn(async move {
                start_countdown(rich_client_clone, chat_id, feedback_msg_id, debounce_secs, buffer_clone).await;
            });
        }
        return Ok(());
    } else {
        if let Some(ref mut active) = *buf {
            active.close_handle.abort();

            active.message_sequence += 1;
            let seq = active.message_sequence;
            active.message_ids.insert(msg.id, seq);

            let content = tokio::fs::read_to_string(&active.file_path).await?;
            let new_content = format::append_message(&content, seq, text);
            tokio::fs::write(&active.file_path, &new_content).await?;

            active.message_count += 1;
            let feedback_msg_id = active.feedback_msg_id;
            let buffer_clone = buffer.clone();
            let debounce_secs = config.debounce_secs;
            let chat_id = config.chat_id;
            let rich_client_clone = rich_client.clone();

            active.close_handle = tokio::spawn(async move {
                start_countdown(rich_client_clone, chat_id, feedback_msg_id, debounce_secs, buffer_clone).await;
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

    let file_content = format!("{}---1---\n\n{}\n", frontmatter, content);
    tokio::fs::write(&file_path, &file_content).await?;

    let chat_id = config.chat_id;
    let thread_id = config.thread_ids.quick_notes;
    let file_content_for_render = tokio::fs::read_to_string(&file_path).await?;
    let render_content = render_for_display(&file_content_for_render);

    let rich_msg_id = rich_client
        .open_window(chat_id, thread_id, config.debounce_secs, &render_content)
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;

    let feedback_msg_id = MessageId(rich_msg_id);
    let buffer_clone = buffer.clone();
    let debounce_secs = config.debounce_secs;
    let rich_client_clone = rich_client.clone();

    let close_handle = tokio::spawn(async move {
        start_countdown(rich_client_clone, chat_id, feedback_msg_id, debounce_secs, buffer_clone).await;
    });

    let mut message_ids = HashMap::new();
    message_ids.insert(msg.id, 1);

    *buf = Some(InboxBuffer {
        file_path,
        close_handle,
        feedback_msg_id,
        message_count: 1,
        message_sequence: 1,
        message_ids,
    });

    Ok(())
}

async fn start_countdown(
    rich_client: RichClient,
    chat_id: i64,
    msg_id: MessageId,
    debounce_secs: u64,
    buffer: ActiveBuffer,
) {
    let file_path = {
        let buf = buffer.lock().await;
        buf.as_ref().map(|b| b.file_path.clone())
    };

    let Some(file_path) = file_path else { return };

    let filename = file_path.file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown")
        .to_string();

    for remaining in (1..debounce_secs).rev() {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
            let render_content = render_for_display(&content);
            let _ = rich_client.update_window(chat_id, msg_id.0, Some(remaining), None, &render_content).await;
        }
    }

    if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
        let render_content = render_for_display(&content);
        let _ = rich_client.update_window(chat_id, msg_id.0, None, Some(&filename), &render_content).await;
    }

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

fn strip_frontmatter(content: &str) -> &str {
    content
        .strip_prefix("---\n")
        .and_then(|s| s.find("\n---\n"))
        .map(|end| &content[end + 5..])
        .unwrap_or(content)
}

fn render_for_display(content: &str) -> String {
    let without_frontmatter = strip_frontmatter(content);
    let re = regex::Regex::new(r"(?m)^---\d+---").unwrap();
    re.replace_all(without_frontmatter, "").to_string()
}

async fn handle_callback(
    _bot: Bot,
    query: teloxide::types::CallbackQuery,
    config: Arc<Config>,
    buffer: ActiveBuffer,
) {
    let data = match query.data.as_deref() {
        Some(d) => d,
        None => return,
    };

    if data != "close_window" {
        return;
    }

    let rich_client = RichClient::new(&config);
    let callback_id = query.id.to_string();
    let _ = rich_client.answer_callback(&callback_id).await;

    let mut buf = buffer.lock().await;
    if let Some(active) = buf.take() {
        active.close_handle.abort();

        let chat_id = config.chat_id;
        let msg_id = query.message.as_ref().map(|m| m.id().0).unwrap_or(0);
        let filename = active.file_path.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_content = tokio::fs::read_to_string(&active.file_path).await.unwrap_or_default();
        let render_content = render_for_display(&file_content);
        let final_content = format!("Файл сохранён: {}\n\n{}", filename, render_content);

        let _ = rich_client.update_window(chat_id, msg_id, None, Some(&filename), &final_content).await;
    }
}
