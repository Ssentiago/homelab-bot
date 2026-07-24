#![allow(dead_code)]

use std::collections::HashMap;
use std::time::{Duration, Instant};
use teloxide::types::MessageId;

struct DedupEntry {
    message_id: MessageId,
    count: u32,
    first_seen: Instant,
}

pub struct DedupCache {
    entries: HashMap<String, DedupEntry>,
    window: Duration,
}

pub enum DedupAction {
    NewMessage,
    Update(MessageId, u32),
}

impl DedupCache {
    pub fn new(window: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            window,
        }
    }

    pub fn check(&mut self, source: &str, level: &str, message: &str) -> DedupAction {
        let key = format!("{}|{}|{}", source, level, message);
        let now = Instant::now();

        if let Some(entry) = self.entries.get_mut(&key)
            && now.duration_since(entry.first_seen) < self.window
        {
            entry.count += 1;
            return DedupAction::Update(entry.message_id, entry.count);
        }

        DedupAction::NewMessage
    }

    pub fn insert(&mut self, source: &str, level: &str, message: &str, msg_id: MessageId) {
        let key = format!("{}|{}|{}", source, level, message);
        self.entries.insert(key, DedupEntry {
            message_id: msg_id,
            count: 1,
            first_seen: Instant::now(),
        });
    }
}
