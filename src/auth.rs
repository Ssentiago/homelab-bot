use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

/// Inline shell helper that emits
///   protocol=https  host=github.com  username=x-access-token  password=$GH_TOKEN
/// to git's credential query on stdin when git asks for credentials
/// for an HTTPS fetch/push. The same shell function is passed to
/// `git ... -c credential.helper="<this>"`. When git later needs
/// credentials it invokes `/bin/sh -c "<this>"` with the verb as
/// `$1` (`get`/`store`/`erase`); on `get` the function echoes two
/// lines which git's credential subsystem parses as the username
/// and password it should send.
///
/// Auth is *only* sourced from this helper: there is no
/// http.extraHeader, no URL-embedded token, no system or global
/// git credential helper (the leading `-c credential.helper=`
/// clears whatever was inherited from system/global/local config).
///
/// Token stays in the bot's process env (GH_TOKEN), placed there by
/// dotenvy from .env; child git inherits it via fork+exec; /bin/sh
/// expands `$GH_TOKEN` at call time. Nothing about the token ever
/// sits on disk or in argv.
const CRED_HELPER: &str = r#"credential.helper=!f() { test "$1" = get && echo username=x-access-token && echo "password=$GH_TOKEN"; }; f"#;

/// Wraps `Command::new("git")` with the two -c args plus -C and the
/// requested argv; sizes and runs to completion, returning
/// trimmed stderr from any failure.
pub async fn run_git(dir: &Path, argv: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        .args(["-c", "credential.helper=", "-c", CRED_HELPER])
        .args(argv)
        .stdin(Stdio::null());
    let out = cmd.output().await.map_err(|e| format!("spawn: {}", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}
