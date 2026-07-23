mod config;
mod modules;
mod router;
mod startup;
mod supervisor;

use std::path::PathBuf;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{MessageId, ThreadId};
use tracing::{info, error};
use tracing_subscriber::EnvFilter;

use config::Config;

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

    let mut router = router::Router::new();

    let quick_notes_rx = if let Some(thread_id) = config.thread_ids.quick_notes {
        let thread_id = ThreadId(MessageId(thread_id));
        Some(router.register(thread_id))
    } else {
        None
    };

    let bot_clone = bot.clone();
    let router_task = tokio::spawn(async move {
        router.run(bot_clone).await;
    });

    let bot_clone2 = bot.clone();
    let config_clone2 = config.clone();

    let _quick_notes_task = if let Some(rx) = quick_notes_rx {
        let bot = bot_clone2.clone();
        let config = config_clone2.clone();
        let buffer = modules::quick_notes::handler::new_buffer();
        Some(tokio::spawn(async move {
            modules::quick_notes::handler::run(bot, config, buffer, rx).await;
        }))
    } else {
        None
    };

    let bot_clone3 = bot.clone();
    let config_clone3 = config.clone();

    let http_task = tokio::spawn(supervisor::run_supervised("http_server", move || {
        let bot = bot_clone3.clone();
        let config = config_clone3.clone();
        async move {
            modules::http_notifications_server::run(bot, config).await;
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
