#![allow(dead_code)]

use sqlx::{Row, SqlitePool};
use std::time::Duration;

pub struct HistoryEntry {
    pub id: i64,
    pub module: String,
    pub action: String,
    pub content: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

pub async fn insert(
    pool: &SqlitePool,
    module: &str,
    action: &str,
    content: Option<&str>,
    metadata: Option<&str>,
) {
    let _ = sqlx::query(
        "INSERT INTO action_history (module, action, content, metadata) VALUES (?, ?, ?, ?)",
    )
    .bind(module)
    .bind(action)
    .bind(content)
    .bind(metadata)
    .execute(pool)
    .await;
}

pub async fn query_module(pool: &SqlitePool, module: &str, limit: u32) -> Vec<HistoryEntry> {
    let rows = sqlx::query(
        "SELECT id, module, action, content, metadata, created_at \
         FROM action_history WHERE module = ? ORDER BY created_at DESC LIMIT ?",
    )
    .bind(module)
    .bind(limit as i64)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rows) => rows
            .iter()
            .map(|r| HistoryEntry {
                id: r.get("id"),
                module: r.get("module"),
                action: r.get("action"),
                content: r.get("content"),
                metadata: r.get("metadata"),
                created_at: r.get("created_at"),
            })
            .collect(),
        Err(_) => vec![],
    }
}

pub fn spawn_cleanup(pool: SqlitePool) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            let _ = sqlx::query(
                "DELETE FROM action_history WHERE created_at < datetime('now', '-1 day')",
            )
            .execute(&pool)
            .await;
        }
    });
}
