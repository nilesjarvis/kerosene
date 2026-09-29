use grammers_client::session::storages::SqliteSession;
use grammers_client::{Client, SenderPool};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

const TELEGRAM_SESSION_OPEN_RETRY_ATTEMPTS: usize = 3;
const TELEGRAM_SESSION_OPEN_RETRY_DELAY: Duration = Duration::from_millis(250);
const TELEGRAM_POOL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) fn telegram_fast_session_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("kerosene").join("telegram_fast.session"))
}

// Serializes short-lived client operations (auth, private channel scans)
// against each other; they share one session file with the live feed stream.
fn telegram_client_op_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

pub(super) async fn with_telegram_client<T, F, Fut>(api_id: i32, f: F) -> Result<T, String>
where
    F: FnOnce(Client) -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let _op_guard = telegram_client_op_lock().lock().await;
    let session_path = telegram_fast_session_path()
        .ok_or_else(|| "Could not resolve Kerosene config directory".to_string())?;
    prepare_session_path(&session_path).await?;
    let session = Arc::new(open_telegram_session(&session_path).await?);
    tighten_session_permissions(&session_path);

    let SenderPool {
        runner,
        updates: _,
        handle,
    } = SenderPool::new(session, api_id);
    let client = Client::new(handle.clone());
    let pool_task = tokio::spawn(runner.run());
    let result = f(client).await;
    handle.quit();
    let _ = shutdown_telegram_pool_task(pool_task, TELEGRAM_POOL_SHUTDOWN_TIMEOUT).await;
    tighten_session_permissions(&session_path);
    result
}

async fn shutdown_telegram_pool_task(
    mut pool_task: tokio::task::JoinHandle<()>,
    timeout: Duration,
) -> bool {
    if tokio::time::timeout(timeout, &mut pool_task).await.is_ok() {
        return false;
    }

    pool_task.abort();
    let _ = pool_task.await;
    true
}

// The session file is shared with the live feed stream; transient SQLite lock
// contention is expected, so retry briefly before reporting failure.
pub(super) async fn open_telegram_session(path: &Path) -> Result<SqliteSession, String> {
    let mut attempt = 1;
    loop {
        match SqliteSession::open(path).await {
            Ok(session) => return Ok(session),
            Err(err) => {
                if attempt >= TELEGRAM_SESSION_OPEN_RETRY_ATTEMPTS {
                    return Err(format!("Telegram session open failed: {err}"));
                }
                attempt += 1;
                tokio::time::sleep(TELEGRAM_SESSION_OPEN_RETRY_DELAY).await;
            }
        }
    }
}

pub(super) async fn prepare_session_path(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create Telegram session directory: {e}"))?;
        tighten_directory_permissions(parent);
    }
    Ok(())
}

#[cfg(unix)]
fn tighten_directory_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}

#[cfg(target_os = "windows")]
fn tighten_directory_permissions(path: &Path) {
    let _ = crate::helpers::restrict_path_to_owner(path);
}

#[cfg(not(any(unix, target_os = "windows")))]
fn tighten_directory_permissions(_path: &Path) {}

#[cfg(unix)]
pub(super) fn tighten_session_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    for candidate in session_file_family(path) {
        if candidate.exists() {
            let _ = std::fs::set_permissions(candidate, std::fs::Permissions::from_mode(0o600));
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) fn tighten_session_permissions(path: &Path) {
    for candidate in session_file_family(path) {
        if candidate.exists() {
            let _ = crate::helpers::restrict_path_to_owner(&candidate);
        }
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
pub(super) fn tighten_session_permissions(_path: &Path) {}

pub(super) fn clear_telegram_fast_session_files() -> Result<usize, String> {
    let Some(path) = telegram_fast_session_path() else {
        return Ok(0);
    };
    clear_telegram_fast_session_files_at(&path)
}

pub(crate) fn clear_telegram_fast_session_files_at(path: &Path) -> Result<usize, String> {
    let mut removed = 0;
    let mut errors = Vec::new();
    for candidate in session_file_family(path) {
        match std::fs::remove_file(&candidate) {
            Ok(()) => removed += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!(
                "remove {} failed: {e}",
                redacted_session_file_display(&candidate)
            )),
        }
    }

    if errors.is_empty() {
        Ok(removed)
    } else {
        Err(errors.join("; "))
    }
}

fn session_file_family(path: &Path) -> [PathBuf; 4] {
    [
        path.to_path_buf(),
        path.with_extension("session-shm"),
        path.with_extension("session-wal"),
        path.with_extension("session-journal"),
    ]
}

fn redacted_session_file_display(path: &Path) -> String {
    path.file_name()
        .map(|name| format!("<config-dir>/{}", name.to_string_lossy()))
        .unwrap_or_else(|| "<config-dir>/<session-file>".to_string())
}

#[cfg(test)]
mod tests;
