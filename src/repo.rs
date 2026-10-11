use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tracing::{error, info, warn};

use crate::auth;
use crate::config::Config;

/// True when the local target directory has a working clone of the
/// configured remote repo. Push workers consult this flag before
/// each tick.
pub static REPO_READY: AtomicBool = AtomicBool::new(false);

const PROGRESS_EVERY: Duration = Duration::from_secs(15);
const RETRY_EVERY: Duration = Duration::from_secs(10);
const WATCHDOG_EVERY: Duration = Duration::from_secs(60);

pub struct RepoInputs {
    pub target: PathBuf,
    pub url: String,
    pub gh_token: String,
}

pub fn inputs_from_config(c: &Config) -> RepoInputs {
    RepoInputs {
        target: PathBuf::from(&c.root),
        url: c.notes_repo.clone(),
        gh_token: c.gh_token.clone(),
    }
}

pub fn preflight(inp: &RepoInputs) -> Result<(), String> {
    let git = std::process::Command::new("git")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("git binary not found in PATH: {}", e))?;
    if !git.status.success() {
        return Err(format!(
            "git --version failed: {}",
            String::from_utf8_lossy(&git.stderr)
        ));
    }
    info!("git OK: {}", String::from_utf8_lossy(&git.stdout).trim());

    if !inp.target.is_absolute() {
        return Err(format!("target {:?} must be an absolute path", inp.target));
    }
    if inp.gh_token.is_empty() {
        return Err("GH_TOKEN is empty".into());
    }
    if inp.url.is_empty() {
        return Err("NOTES_REPO is empty".into());
    }
    if !(inp.url.starts_with("https://") || inp.url.starts_with("http://")) {
        return Err(format!(
            "url must be http(s) (got {}); ssh://, git://, file://, git@ are rejected",
            inp.url
        ));
    }
    info!(
        "preflight OK: target={} repo={}",
        inp.target.display(),
        inp.url
    );
    Ok(())
}

/// Background task. State machine:
///   REPO_READY=false -> prime(); on Ok -> REPO_READY=true; on Err
///                       wait RETRY_EVERY then retry.
///   REPO_READY=true  -> sleep WATCHDOG_EVERY; if .git vanished ->
///                       drop the flag and reclone.
pub fn spawn_cloner(inp: RepoInputs) {
    info!(
        "cloner spawned: target={} repo={}",
        inp.target.display(),
        inp.url
    );
    tokio::spawn(async move {
        loop {
            if REPO_READY.load(Ordering::Relaxed) {
                tokio::time::sleep(WATCHDOG_EVERY).await;
                if !inp.target.join(".git").exists() {
                    warn!(
                        ".git vanished at {}; flipping REPO_READY=false to reclone",
                        inp.target.display()
                    );
                    REPO_READY.store(false, Ordering::Relaxed);
                }
                continue;
            }

            let progress = tokio::spawn(progress_log(inp.target.clone()));
            let result = prime(&inp).await;
            progress.abort();
            match result {
                Ok(()) => {
                    info!(
                        "clone established at {} (REPO_READY=true)",
                        inp.target.display()
                    );
                    REPO_READY.store(true, Ordering::Relaxed);
                }
                Err(reason) => {
                    error!(
                        "prime failed: {} (will retry in {:?})",
                        reason, RETRY_EVERY
                    );
                    tokio::time::sleep(RETRY_EVERY).await;
                }
            }
        }
    });
}

/// Bring target to a working clone of url. Auth is purely in-process:
/// every git call goes through crate::auth::run_git which injects
/// `-c credential.helper=` (clears inherited helpers) and an
/// inline shell helper that reads GH_TOKEN from the bot's env at
/// invocation time. The token lives only in the bot's process env;
/// it is never written to .git/, never embedded in any URL, never
/// appears in argv.
async fn prime(inp: &RepoInputs) -> Result<(), String> {
    tokio::fs::create_dir_all(&inp.target)
        .await
        .map_err(|e| format!("mkdir target {:?}: {}", inp.target, e))?;

    auth::run_git(&inp.target, &["init", "-b", "master"]).await?;

    let _ = auth::run_git(&inp.target, &["remote", "remove", "origin"]).await;
    auth::run_git(&inp.target, &["remote", "add", "origin", &inp.url]).await?;

    auth::run_git(&inp.target, &["fetch", "origin", "master"]).await?;
    auth::run_git(&inp.target, &["reset", "--hard", "origin/master"]).await?;

    tokio::fs::create_dir_all(inp.target.join("00 Inbox"))
        .await
        .map_err(|e| format!("mkdir 00 Inbox: {}", e))?;
    Ok(())
}

/// Reports on-disk size of `target` every PROGRESS_EVERY while alive.
async fn progress_log(target: PathBuf) {
    let mut tick = tokio::time::interval(PROGRESS_EVERY);
    tick.tick().await;
    loop {
        tick.tick().await;
        let path = target.clone();
        let bytes = tokio::task::spawn_blocking(move || disk_size_bytes(&path))
            .await
            .ok()
            .and_then(Result::ok);
        match bytes {
            Some(b) => info!(
                "clone progress at {}: {:.2} MiB",
                target.display(),
                b as f64 / 1_048_576.0
            ),
            None => info!("clone progress at {}: stat failed", target.display()),
        }
    }
}

fn disk_size_bytes(root: &Path) -> std::io::Result<u64> {
    let out = std::process::Command::new("du")
        .arg("-sb")
        .arg(root)
        .output()?;
    if !out.status.success() {
        return Err(std::io::Error::other("du exited non-zero"));
    }
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .and_then(|n| n.parse::<u64>().ok())
        .ok_or_else(|| std::io::Error::other("du parse failed"))
}
