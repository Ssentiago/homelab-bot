use std::sync::{Arc, Mutex};
use std::time::Duration;

use teloxide::prelude::*;
use tokio::time::Instant;
use tracing::error;

pub type HealthCheck = Box<dyn Fn() -> Result<(), String> + Send + Sync>;

pub struct ModuleHandle {
    pub name: String,
    pub last_alive: Arc<Mutex<Instant>>,
    pub check: HealthCheck,
}

pub struct Heartbeat {
    pub start: Instant,
    pub last_sync: Mutex<Option<Instant>>,
    pub modules: Mutex<Vec<ModuleHandle>>,
}

impl Heartbeat {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            last_sync: Mutex::new(Some(Instant::now())),
            modules: Mutex::new(Vec::new()),
        }
    }

    pub fn register(&self, handle: ModuleHandle) {
        self.modules.lock().unwrap().push(handle);
    }
}

pub fn spawn(bot: Bot, hb: std::sync::Arc<Heartbeat>) {
    let bot_clone = bot.clone();
    let hb_clone = hb.clone();
    tokio::spawn(async move {
        update_description(&bot_clone, &hb_clone).await;
    });

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            update_description(&bot, &hb).await;
        }
    });
}

async fn update_description(bot: &Bot, hb: &Heartbeat) {
    let uptime = hb.start.elapsed();
    let hours = uptime.as_secs() / 3600;
    let mins = (uptime.as_secs() % 3600) / 60;

    let sync_str = match *hb.last_sync.lock().unwrap() {
        Some(t) => {
            let ago = t.elapsed();
            if ago.as_secs() < 120 {
                format!("{}s ago", ago.as_secs())
            } else {
                format!("{}m ago", ago.as_secs() / 60)
            }
        }
        None => "never".to_string(),
    };

    let modules_str = {
        let modules = hb.modules.lock().unwrap();
        let mut lines: Vec<String> = modules
            .iter()
            .map(|h| {
                let alive_age = h.last_alive.lock().unwrap().elapsed().as_secs();
                if alive_age > 120 {
                    format!("  {} 💀", h.name)
                } else {
                    match (h.check)() {
                        Ok(()) => format!("  {} ✓", h.name),
                        Err(e) => format!("  {} ⚠ {}", h.name, e),
                    }
                }
            })
            .collect();
        lines.sort_by(|a, b| a[2..].cmp(&b[2..]));
        if lines.is_empty() {
            String::new()
        } else {
            format!("\n{}", lines.join("\n"))
        }
    };

    let desc = format!(
        "Uptime: {}h {}m · Sync: {} · v{}{}",
        hours, mins, sync_str, env!("CARGO_PKG_VERSION"), modules_str
    );

    if let Err(e) = bot.set_my_short_description().short_description(desc).await {
        error!("Failed to update short description: {}", e);
    } else {
        hb.last_sync.lock().unwrap().replace(Instant::now());
    }
}
