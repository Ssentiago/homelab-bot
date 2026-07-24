#![allow(dead_code)]

use sqlx::SqlitePool;

pub struct TaskQueue {
    pool: SqlitePool,
    table: String,
}

impl TaskQueue {
    pub fn new(pool: SqlitePool, table: &str) -> Self {
        Self {
            pool,
            table: table.to_string(),
        }
    }

    pub async fn init_table(&self, extra_columns: &str) -> Result<(), sqlx::Error> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                status TEXT NOT NULL DEFAULT 'pending',
                attempts INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 5,
                last_error TEXT,
                next_attempt_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                {}
            )",
            self.table, extra_columns
        );
        sqlx::query(&sql).execute(&self.pool).await?;

        let idx = format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_status ON {}(status)",
            self.table, self.table
        );
        sqlx::query(&idx).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn claim_next(&self) -> Result<Option<sqlx::sqlite::SqliteRow>, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        let sql = format!(
            "UPDATE {}
             SET status = 'processing', updated_at = ?
             WHERE id = (
                 SELECT id FROM {}
                 WHERE status = 'pending' AND attempts < max_attempts
                 AND (next_attempt_at IS NULL OR next_attempt_at <= ?)
                 ORDER BY created_at
                 LIMIT 1
             )
             RETURNING *",
            self.table, self.table
        );

        let row = sqlx::query(&sql)
            .bind(&now)
            .bind(&now)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row)
    }

    pub async fn mark_done(&self, id: i64) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let sql = format!(
            "UPDATE {} SET status = 'done', updated_at = ? WHERE id = ?",
            self.table
        );
        sqlx::query(&sql)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_pending_retry(&self, id: i64, error: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let sql = format!(
            "UPDATE {}
             SET status = 'pending', attempts = attempts + 1, last_error = ?, updated_at = ?
             WHERE id = ?",
            self.table
        );
        sqlx::query(&sql)
            .bind(error)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: i64, error: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let sql = format!(
            "UPDATE {}
             SET status = 'failed', attempts = attempts + 1, last_error = ?, updated_at = ?
             WHERE id = ?",
            self.table
        );
        sqlx::query(&sql)
            .bind(error)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
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
