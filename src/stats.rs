use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::Row;
use tokio::time::Instant;

pub struct BotStart(pub Instant);

impl BotStart {
    pub fn now() -> Self {
        Self(Instant::now())
    }
}

pub async fn record_stat(pool: &SqlitePool, kind: &str) {
    let _ = sqlx::query("INSERT INTO stats (kind) VALUES (?)")
        .bind(kind)
        .execute(pool)
        .await;
}

pub struct StatsSnapshot {
    pub notes: u32,
    pub notifications: u32,
    pub dedup_suppressed: u32,
}

pub async fn query_today(pool: &SqlitePool) -> StatsSnapshot {
    let rows = sqlx::query(
        "SELECT kind, COUNT(*) as count FROM stats WHERE created_at >= date('now') GROUP BY kind"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut notes = 0u32;
    let mut notifications = 0u32;
    let mut dedup = 0u32;

    for row in rows {
        let kind: String = row.get("kind");
        let count: i64 = row.get("count");
        match kind.as_str() {
            "note" => notes = count as u32,
            "notification" => notifications = count as u32,
            "dedup_suppressed" => dedup = count as u32,
            _ => {}
        }
    }

    StatsSnapshot { notes, notifications, dedup_suppressed: dedup }
}

pub fn format_status(snapshot: &StatsSnapshot, start: &BotStart) -> String {
    let uptime = start.0.elapsed();
    let hours = uptime.as_secs() / 3600;
    let mins = (uptime.as_secs() % 3600) / 60;

    format!(
        "📊 Homelab Bot\n\n\
         Аптайм: {}ч {}мин\n\n\
         Быстрые заметки\n\
         • Сообщений за сегодня: {}\n\n\
         Уведомления\n\
         • Отправлено: {}\n\
         • Дедуп: {}",
        hours, mins, snapshot.notes, snapshot.notifications, snapshot.dedup_suppressed
    )
}

pub async fn cleanup_expired(pool: &SqlitePool) {
    if let Err(e) = sqlx::query("DELETE FROM stats WHERE created_at < date('now')")
        .execute(pool)
        .await
    {
        tracing::error!("Failed to clean expired stats at startup: {}", e);
    }
}

pub fn spawn_cleanup(pool: SqlitePool) {
    tokio::spawn(async move {
        loop {
            let now = chrono::Local::now();
            let midnight = now.date_naive()
                .succ_opt()
                .unwrap()
                .and_hms_opt(0, 0, 5)
                .unwrap();
            let duration = (midnight - now.naive_local()).to_std().unwrap_or(Duration::from_secs(60));
            tokio::time::sleep(duration).await;

            if let Err(e) = sqlx::query("DELETE FROM stats WHERE created_at < date('now')")
                .execute(&pool)
                .await
            {
                tracing::error!("Failed to clean expired stats: {}", e);
            }
        }
    });
}
