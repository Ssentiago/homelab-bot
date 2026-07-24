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
use std::sync::Arc;
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
        if let Err(e) = self_update() {
            eprintln!("Update failed: {}", e);
            std::process::exit(1);
        }
        return;
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
    heartbeat.register(ModuleHandle {
        name: "notifications".into(),
        last_alive: nf_alive.clone(),
        check: Box::new(move || {
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
        async move {
            modules::http_notifications_server::run(bot, config, queue, alive).await;
        }
    }));

    let _ = tokio::join!(router_task, http_task);
}

fn self_update() -> Result<(), Box<dyn std::error::Error>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("Ssentiago")
        .repo_name("homelab-bot")
        .bin_name("homelab-bot")
        .show_download_progress(true)
        .no_confirm(true)
        .current_version(self_update::cargo_crate_version!())
        .asset_identifier("homelab-bot")
        .build()?
        .update()?;
    println!("Updated to version {}", status.version());
    Ok(())
}
