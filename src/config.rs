use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub chat_id: i64,
    pub notifications_thread_id: Option<i32>,
    pub quick_notes_thread_id: Option<i32>,
    pub notify_server_port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bot_token: env::var("BOT_TOKEN").expect("BOT_TOKEN must be set"),
            chat_id: env::var("CHAT_ID")
                .expect("CHAT_ID must be set")
                .parse()
                .expect("CHAT_ID must be a valid i64"),
            notifications_thread_id: env::var("NOTIFICATIONS_THREAD_ID")
                .ok()
                .filter(|v| !v.is_empty())
                .map(|v| v.parse().expect("NOTIFICATIONS_THREAD_ID must be a valid i32")),
            quick_notes_thread_id: env::var("QUICK_NOTES_THREAD_ID")
                .ok()
                .filter(|v| !v.is_empty())
                .map(|v| v.parse().expect("QUICK_NOTES_THREAD_ID must be a valid i32")),
            notify_server_port: env::var("NOTIFY_SERVER_PORT")
                .unwrap_or_else(|_| "8787".to_string())
                .parse()
                .expect("NOTIFY_SERVER_PORT must be a valid u16"),
        }
    }

    pub fn topics_not_created(&self) -> bool {
        self.notifications_thread_id.is_none() && self.quick_notes_thread_id.is_none()
    }

    pub fn save_thread_ids(&mut self, notifications: i32, quick_notes: i32) {
        Self::update_env("NOTIFICATIONS_THREAD_ID", &notifications.to_string());
        Self::update_env("QUICK_NOTES_THREAD_ID", &quick_notes.to_string());
        self.notifications_thread_id = Some(notifications);
        self.quick_notes_thread_id = Some(quick_notes);
    }

    fn update_env(key: &str, value: &str) {
        let content = std::fs::read_to_string(".env").expect("Failed to read .env");
        let updated = content
            .lines()
            .map(|line| {
                if line.starts_with(&format!("{}=", key)) {
                    format!("{}={}", key, value)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(".env", updated).expect("Failed to write .env");
    }
}
