use std::env;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadIds {
    pub notifications: Option<i32>,
    pub quick_notes: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub chat_id: i64,
    pub thread_ids: ThreadIds,
    pub notify_server_port: u16,
    pub notify_token: String,
    pub root: String,
    pub debounce_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bot_token: env::var("BOT_TOKEN").expect("BOT_TOKEN must be set"),
            chat_id: env::var("CHAT_ID")
                .expect("CHAT_ID must be set")
                .parse()
                .expect("CHAT_ID must be a valid i64"),
            thread_ids: Self::load_thread_ids(),
            notify_server_port: env::var("NOTIFY_SERVER_PORT")
                .unwrap_or_else(|_| "8787".to_string())
                .parse()
                .expect("NOTIFY_SERVER_PORT must be a valid u16"),
            notify_token: env::var("NOTIFY_TOKEN").expect("NOTIFY_TOKEN must be set"),
            root: env::var("NOTES_ROOT").expect("NOTES_ROOT must be set"),
            debounce_secs: env::var("DEBOUNCE_SECS")
                .unwrap_or_else(|_| "45".to_string())
                .parse()
                .expect("DEBOUNCE_SECS must be a valid u64"),
        }
    }

    fn load_thread_ids() -> ThreadIds {
        if !Path::new(CONFIG_FILE).exists() {
            return ThreadIds {
                notifications: None,
                quick_notes: None,
            };
        }

        let content = fs::read_to_string(CONFIG_FILE).expect("Failed to read config.json");
        serde_json::from_str(&content).expect("Failed to parse config.json")
    }

    pub fn topics_not_created(&self) -> bool {
        self.thread_ids.notifications.is_none() && self.thread_ids.quick_notes.is_none()
    }

    pub fn save_thread_ids(&mut self, notifications: i32, quick_notes: i32) {
        let thread_ids = ThreadIds {
            notifications: Some(notifications),
            quick_notes: Some(quick_notes),
        };

        let content = serde_json::to_string_pretty(&thread_ids).expect("Failed to serialize thread_ids");
        fs::write(CONFIG_FILE, content).expect("Failed to write config.json");

        self.thread_ids = thread_ids;
    }
}
