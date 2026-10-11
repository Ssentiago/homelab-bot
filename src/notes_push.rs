use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use sqlx::{Row, SqlitePool};
use tokio::process::Command;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::repo::REPO_READY;

const TICK_SECS: u64 = 30;

pub fn spawn_push_worker(config: Arc<Config>, pool: SqlitePool) {
    info!(
        "Notes push worker started: interval={}s target={}",
        TICK_SECS, config.root
    );
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(TICK_SECS)).await;

            if !REPO_READY.load(std::sync::atomic::Ordering::Relaxed) {
                warn!("Notes push worker tick: REPO_READY=false, skipping (cloner still working)");
                continue;
            }

            let rows = match sqlx::query(
                "SELECT path FROM note_push_tasks ORDER BY created_at",
            )
                .fetch_all(&pool)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    error!("Notes push worker tick: select failed: {}", e);
                    continue;
                }
            };
            if rows.is_empty() {
                continue;
            }
            info!("Notes push worker tick: draining {} task(s)", rows.len());

            for row in &rows {
                let path: String = row.get("path");
                match push_one(&config.root, &path).await {
                    PushOutcome::Pushed => {
                        info!("Notes push worker: OK pushed {}", path);
                    }
                    PushOutcome::AlreadyUpToDate => {
                        info!(
                            "Notes push worker: {} already in remote (no diff after add), dropping task",
                            path
                        );
                    }
                    PushOutcome::Failed(reason, stderr) => {
                        error!(
                            "Notes push worker: {} left in queue (will retry next tick): {}: {}",
                            path, reason, stderr
                        );
                        continue;
                    }
                }
                if let Err(e) = sqlx::query("DELETE FROM note_push_tasks WHERE path = ?")
                    .bind(&path)
                    .execute(&pool)
                    .await
                {
                    error!("Notes push worker: delete task {} failed: {}", path, e);
                }
            }
        }
    });
}

enum PushOutcome {
    Pushed,
    AlreadyUpToDate,
    Failed(String, String),
}

async fn push_one(root: &str, path: &str) -> PushOutcome {
    let run = |argv: &[&str]| {
        let mut cmd = Command::new("git");
        cmd.args(argv).current_dir(root).stdin(Stdio::null());
        async move {
            cmd.output()
                .await
                .map_err(|e: std::io::Error| format!("spawn: {}", e))
        }
    };

    match run(&["pull", "--rebase", "origin", "master"]).await {
        Ok(o) if o.status.success() => {}
        Ok(o) => {
            return PushOutcome::Failed(
                "pull".into(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
            )
        }
        Err(e) => return PushOutcome::Failed("pull".into(), e),
    }

    match run(&["add", path]).await {
        Ok(o) if o.status.success() => {}
        Ok(o) => {
            return PushOutcome::Failed(
                "add".into(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
            )
        }
        Err(e) => return PushOutcome::Failed("add".into(), e),
    }

    match run(&["diff", "--cached", "--quiet"]).await {
        Ok(o) if o.status.success() => return PushOutcome::AlreadyUpToDate,
        _ => {}
    }

    let msg = format!(
        "bot: note {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    match run(&["commit", "-m", &msg]).await {
        Ok(o) if o.status.success() => {}
        Ok(o) => {
            return PushOutcome::Failed(
                "commit".into(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
            )
        }
        Err(e) => return PushOutcome::Failed("commit".into(), e),
    }

    match run(&["push", "origin", "master"]).await {
        Ok(o) if o.status.success() => PushOutcome::Pushed,
        Ok(o) => PushOutcome::Failed(
            "push".into(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        ),
        Err(e) => PushOutcome::Failed("push".into(), e),
    }
}
