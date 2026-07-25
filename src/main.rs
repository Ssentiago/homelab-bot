mod config;
mod db;
mod dedup;
mod heartbeat;
mod modules;
mod queue;
mod router;
mod startup;
mod stats;
mod supervisor;
mod worker;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde::Deserialize;
use teloxide::prelude::*;
use teloxide::types::{MessageId, ThreadId};
use tracing::{info, error};
use tracing_subscriber::EnvFilter;

use config::Config;
use heartbeat::{Heartbeat, ModuleHandle};
use stats::BotStart;

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls ring provider");

    if std::env::args().any(|arg| arg == "--update") {
        match self_update() {
            Err(e) => {
                eprintln!("Update failed: {}", e);
                std::process::exit(1);
            }
            Ok(true) => std::process::exit(0),
            Ok(false) => return,
        }
    }

    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let mut config = Config::from_env();

    let root = PathBuf::from(&config.root);
    if !root.is_dir() {
        panic!(
            "NOTES_ROOT path {:?} does not exist or is not a directory. \
             Create it manually before starting the bot.",
            root
        );
    }

    let bot = Bot::new(&config.bot_token);

    info!("Homelab Bot starting...");
    info!("Chat ID: {}", config.chat_id);

    if let Err(e) = startup::ensure_topics_exist(&bot, &mut config).await {
        error!("Failed to create topics: {}", e);
        std::process::exit(1);
    }

    let config = Arc::new(config);
    info!("Notifications thread: {:?}", config.thread_ids.notifications);
    info!("Quick notes thread: {:?}", config.thread_ids.quick_notes);

    let pool = db::init_db().await;

    stats::cleanup_expired(&pool).await;

    let bot_start = Arc::new(BotStart::now());

    let heartbeat = Arc::new(Heartbeat::new());

    // Register quick_notes health check
    let qn_alive = Arc::new(std::sync::Mutex::new(tokio::time::Instant::now()));
    let qn_root = config.root.clone();
    heartbeat.register(ModuleHandle {
        name: "quick_notes".into(),
        last_alive: qn_alive.clone(),
        check: Box::new(move || {
            let root = std::path::Path::new(&qn_root);
            if !root.is_dir() {
                return Err("NOTES_ROOT missing".into());
            }
            let test = root.join(".health_check");
            std::fs::write(&test, b"").map_err(|e| format!("disk: {}", e))?;
            std::fs::remove_file(&test).ok();
            Ok(())
        }),
    });

    // Register notifications health check
    let nf_alive = Arc::new(std::sync::Mutex::new(tokio::time::Instant::now()));
    let nf_port = config.notify_server_port;
    let nf_ready = Arc::new(AtomicBool::new(false));
    let nf_ready_check = nf_ready.clone();
    heartbeat.register(ModuleHandle {
        name: "notifications".into(),
        last_alive: nf_alive.clone(),
        check: Box::new(move || {
            if !nf_ready_check.load(Ordering::Relaxed) {
                return Err("starting".into());
            }
            let addr = format!("127.0.0.1:{}", nf_port);
            std::net::TcpStream::connect_timeout(
                &addr.parse().map_err(|e| format!("parse: {}", e))?,
                std::time::Duration::from_secs(2),
            )
            .map(|_| ())
            .map_err(|e| format!("port: {}", e))
        }),
    });

    heartbeat::spawn(bot.clone(), heartbeat.clone());

    stats::spawn_cleanup(pool.clone());

    let notifications_queue = queue::TaskQueue::new(pool.clone(), "pending_notifications");
    notifications_queue.init_table("chat_id INTEGER NOT NULL, thread_id INTEGER, kind TEXT NOT NULL DEFAULT 'plain', text TEXT, rich_markdown TEXT, edit_message_id INTEGER").await.expect("Failed to init notifications table");

    let worker_pool = pool.clone();
    worker::spawn_notification_worker(queue::TaskQueue::new(worker_pool, "pending_notifications"), bot.clone(), config.clone());

    let mut router = router::Router::new();

    let quick_notes_rx = if let Some(thread_id) = config.thread_ids.quick_notes {
        let thread_id = ThreadId(MessageId(thread_id));
        Some(router.register(thread_id))
    } else {
        None
    };

    let callback_rx = router.register_callback();

    let bot_clone = bot.clone();
    let router_pool = pool.clone();
    let router_start = bot_start.clone();
    let router_task = tokio::spawn(async move {
        router.run(bot_clone, router_pool, router_start).await;
    });

    let bot_clone2 = bot.clone();
    let config_clone2 = config.clone();
    let notes_pool = pool.clone();

    let _quick_notes_task = if let Some(rx) = quick_notes_rx {
        let bot = bot_clone2.clone();
        let config = config_clone2.clone();
        let buffer = modules::quick_notes::handler::new_buffer();
        Some(tokio::spawn(async move {
            modules::quick_notes::handler::run(bot, config, notes_pool, qn_alive.clone(), buffer, rx, callback_rx).await;
        }))
    } else {
        None
    };

    let bot_clone3 = bot.clone();
    let config_clone3 = config.clone();
    let server_pool = pool.clone();

    let http_task = tokio::spawn(supervisor::run_supervised("http_server", move || {
        let bot = bot_clone3.clone();
        let config = config_clone3.clone();
        let queue = queue::TaskQueue::new(server_pool.clone(), "pending_notifications");
        let alive = nf_alive.clone();
        let ready = nf_ready.clone();
        async move {
            modules::http_notifications_server::run(bot, config, queue, alive, ready).await;
        }
    }));

    let _ = tokio::join!(router_task, http_task);
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

fn self_update() -> Result<bool, Box<dyn std::error::Error>> {
    let current = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", current);

    println!("Checking for updates...");
    let client = reqwest::blocking::Client::new();
    let release: GitHubRelease = client
        .get("https://api.github.com/repos/Ssentiago/homelab-bot/releases/latest")
        .header("User-Agent", "homelab-bot-updater")
        .header("Accept", "application/vnd.github+json")
        .send()?
        .json()?;

    let latest = &release.tag_name;
    println!("Latest version: {}", latest);

    if current == latest {
        println!("Already up to date!");
        return Ok(false);
    }

    println!("New version found! {} -> {}", current, latest);

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == "homelab-bot")
        .ok_or("Asset 'homelab-bot' not found in release")?;

    println!("Downloading {} bytes...", asset.size);
    let response = reqwest::blocking::get(&asset.browser_download_url)?;
    let bytes = response.bytes()?;
    println!("Downloaded successfully");

    println!("Installing update...");
    let new_path = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .with_file_name("homelab-bot.new");
    std::fs::write(&new_path, &bytes)?;

    self_replace::self_replace(&new_path)?;
    let _ = std::fs::remove_file(&new_path);

    println!("Updated to {}!", latest);
    Ok(true)
}
