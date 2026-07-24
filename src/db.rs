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

    info!("Database initialized");
    pool
}
