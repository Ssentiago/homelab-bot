#![allow(dead_code)]

use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct PendingNotification {
    pub id: i64,
    pub chat_id: i64,
    pub thread_id: Option<i32>,
    pub kind: String,
    pub text: Option<String>,
    pub rich_markdown: Option<String>,
    pub edit_message_id: Option<i32>,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_error: Option<String>,
    pub next_attempt_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn insert_notification(
    pool: &SqlitePool,
    chat_id: i64,
    thread_id: Option<i32>,
    kind: &str,
    text: Option<&str>,
    rich_markdown: Option<&str>,
    edit_message_id: Option<i32>,
) -> Result<i64, sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO pending_notifications (chat_id, thread_id, kind, text, rich_markdown, edit_message_id, status, attempts, max_attempts, next_attempt_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 'pending', 0, 5, ?, ?, ?)",
    )
    .bind(chat_id)
    .bind(thread_id)
    .bind(kind)
    .bind(text)
    .bind(rich_markdown)
    .bind(edit_message_id)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn claim_next_pending(pool: &SqlitePool) -> Result<Option<PendingNotification>, sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();

    let row = sqlx::query_as::<_, PendingNotification>(
        "UPDATE pending_notifications
         SET status = 'processing', updated_at = ?
         WHERE id = (
             SELECT id FROM pending_notifications
             WHERE status = 'pending' AND attempts < max_attempts
             AND (next_attempt_at IS NULL OR next_attempt_at <= ?)
             ORDER BY created_at
             LIMIT 1
         )
         RETURNING *",
    )
    .bind(&now)
    .bind(&now)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn mark_done(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE pending_notifications SET status = 'done', updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_pending_retry(pool: &SqlitePool, id: i64, error: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE pending_notifications
         SET status = 'pending', attempts = attempts + 1, last_error = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(error)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failed(pool: &SqlitePool, id: i64, error: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE pending_notifications
         SET status = 'failed', attempts = attempts + 1, last_error = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(error)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
