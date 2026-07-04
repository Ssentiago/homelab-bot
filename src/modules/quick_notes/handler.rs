use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Local;
use teloxide::prelude::*;
use teloxide::types::{ThreadId, MessageId};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::config::Config;

const TITLE_MAX_LEN: usize = 50;

pub struct InboxBuffer {
    file_path: PathBuf,
    close_handle: JoinHandle<()>,
}

pub type ActiveBuffer = Arc<Mutex<Option<InboxBuffer>>>;

pub fn new_buffer() -> ActiveBuffer {
    Arc::new(Mutex::new(None))
}

pub async fn run(bot: Bot, config: Arc<Config>, buffer: ActiveBuffer) {
    println!("Quick notes task started");

    let quick_notes_thread_id = config
        .thread_ids
        .quick_notes
        .map(|id| ThreadId(MessageId(id)));

    let handler = move |bot: Bot, msg: Message| {
        let config = config.clone();
        let buffer = buffer.clone();
        let thread_id = quick_notes_thread_id;
        async move {
            if let Some(thread_id) = thread_id {
                if msg.thread_id == Some(thread_id) {
                    handle_message(bot, msg, config, buffer).await?;
                }
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
    _bot: Bot,
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
        // Buffer is active - append to existing file
        active.close_handle.abort();

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&active.file_path)
            .expect("Failed to open note file");

        use std::io::Write;
        writeln!(file, "\n\n{}\n", text).expect("Failed to write to note file");

        let buffer_clone = buffer.clone();
        let debounce_secs = config.debounce_secs;

        active.close_handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(debounce_secs)).await;
            let mut buf = buffer_clone.lock().await;
            *buf = None;
        });
    } else {
        // Buffer is empty - create new file
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

        std::fs::write(&file_path, format!("{}{}\n", frontmatter, text))
            .expect("Failed to write note file");

        let buffer_clone = buffer.clone();
        let debounce_secs = config.debounce_secs;

        let close_handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(debounce_secs)).await;
            let mut buf = buffer_clone.lock().await;
            *buf = None;
        });

        *buf = Some(InboxBuffer {
            file_path,
            close_handle,
        });
    }

    Ok(())
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
    match s.char_indices().take(max_len).last() {
        Some((idx, _)) => {
            let truncated = &s[..idx];
            match truncated.rfind(' ') {
                Some(space_idx) => &truncated[..space_idx],
                None => truncated,
            }
        }
        None => s,
    }
}
