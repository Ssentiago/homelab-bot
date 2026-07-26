use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tracing::info;

const DB_URL: &str = "sqlite:homelab.db?mode=rwc";

pub async fn init_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect(DB_URL)
        .await
        .expect("Failed to connect to SQLite");

    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .expect("Failed to set WAL mode");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pending_notifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_id INTEGER NOT NULL,
            thread_id INTEGER,
            kind TEXT NOT NULL DEFAULT 'plain',
            text TEXT,
            rich_markdown TEXT,
            edit_message_id INTEGER,
            status TEXT NOT NULL DEFAULT 'pending',
            attempts INTEGER NOT NULL DEFAULT 0,
            max_attempts INTEGER NOT NULL DEFAULT 5,
            last_error TEXT,
            next_attempt_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create pending_notifications table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pending_notifications_status ON pending_notifications(status)")
        .execute(&pool)
        .await
        .expect("Failed to create index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stats (
            kind TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create stats table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_stats_kind_created ON stats(kind, created_at)")
        .execute(&pool)
        .await
        .expect("Failed to create stats index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS persistent_ui (
            key        TEXT PRIMARY KEY DEFAULT 'main',
            message_id INTEGER NOT NULL,
            chat_id    INTEGER NOT NULL,
            thread_id  INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create persistent_ui table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS action_history (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            module     TEXT NOT NULL,
            action     TEXT NOT NULL,
            content    TEXT,
            metadata   TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create action_history table");

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_action_history_module ON action_history(module, created_at)",
    )
    .execute(&pool)
    .await
    .expect("Failed to create action_history index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pending_cleanup (
            chat_id   INTEGER NOT NULL,
            message_id INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create pending_cleanup table");

    info!("Database initialized");
    pool
}
