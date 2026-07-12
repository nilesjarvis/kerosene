use super::normalize_fills;
use crate::api::UserFill;
use crate::config::{user_config_dir, user_config_path};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

static JOURNAL_CACHE_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Journal Cache Persistence
// ---------------------------------------------------------------------------

pub fn load_cache(address: &str) -> Result<Vec<UserFill>, String> {
    let Some(path) = crate::config::journal_cache_path(address) else {
        return Err("No cache path available".to_string());
    };
    if !path.exists() {
        return Err("Cache does not exist".to_string());
    }
    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read cache file: {e}"))?;
    let mut fills: Vec<UserFill> =
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse cache: {e}"))?;
    normalize_fills(&mut fills);
    Ok(fills)
}

pub fn save_cache(address: &str, fills: &[UserFill]) -> Result<(), String> {
    let Some(path) = crate::config::journal_cache_path(address) else {
        return Err("No cache path available".to_string());
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut normalized = fills.to_vec();
    normalize_fills(&mut normalized);

    let json = serde_json::to_string(&normalized)
        .map_err(|e| format!("Failed to serialize cache: {e}"))?;
    let temp_path = unique_temp_cache_path(&path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp_path)
        .map_err(|e| format!("Failed to create temporary cache file: {e}"))?;
    if let Err(e) = file.write_all(json.as_bytes()) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Failed to write temporary cache file: {e}"));
    }
    let _ = file.sync_all();
    drop(file);

    // Windows does not allow rename to replace an existing destination. Stage
    // the old cache beside it so a failed replacement can restore the last
    // good file instead of deleting it first.
    if let Err(rename_err) = replace_cache_file(&temp_path, &path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!(
            "Failed to move cache file into place; previous cache left untouched: {rename_err}"
        ));
    }

    if let Some(parent) = path.parent()
        && let Ok(dir) = std::fs::File::open(parent)
    {
        let _ = dir.sync_all();
    }

    Ok(())
}

pub fn clear_cache(address: &str) -> Result<usize, String> {
    let Some(path) = crate::config::journal_cache_path(address) else {
        return Err("No cache path available".to_string());
    };

    clear_cache_path_family(&path)
}

fn clear_cache_path_family(path: &std::path::Path) -> Result<usize, String> {
    let mut removed = 0;
    let mut errors = Vec::new();

    match remove_file_if_exists(path) {
        Ok(true) => removed += 1,
        Ok(false) => {}
        Err(e) => errors.push(e),
    }

    if let (Some(parent), Some(file_name)) = (
        path.parent(),
        path.file_name().and_then(|name| name.to_str()),
    ) {
        let temp_prefix = format!("{file_name}.tmp.");
        match std::fs::read_dir(parent) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let temp_path = entry.path();
                            let is_temp_cache = temp_path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .is_some_and(|name| name.starts_with(&temp_prefix));

                            if is_temp_cache {
                                match remove_file_if_exists(&temp_path) {
                                    Ok(true) => removed += 1,
                                    Ok(false) => {}
                                    Err(e) => errors.push(e),
                                }
                            }
                        }
                        Err(e) => errors.push(format!(
                            "read journal cache directory {} entry failed: {e}",
                            user_config_dir()
                        )),
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!(
                "read journal cache directory {} failed: {e}",
                user_config_dir()
            )),
        }
    }

    if errors.is_empty() {
        Ok(removed)
    } else {
        Err(errors.join("; "))
    }
}

fn remove_file_if_exists(path: &std::path::Path) -> Result<bool, String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("remove {} failed: {e}", user_config_path(path))),
    }
}

fn unique_temp_cache_path(path: &std::path::Path) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = JOURNAL_CACHE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!(
        "json.tmp.{}.{}.{}",
        std::process::id(),
        nanos,
        counter
    ))
}

fn replace_cache_file(temp_path: &std::path::Path, path: &std::path::Path) -> Result<(), String> {
    match std::fs::rename(temp_path, path) {
        Ok(()) => Ok(()),
        Err(first_error) if cfg!(windows) && path.is_file() => {
            let rollback_path = unique_cache_sidecar_path(path, "replace-old");
            std::fs::rename(path, &rollback_path)
                .map_err(|error| format!("stage existing cache failed: {error}"))?;

            match std::fs::rename(temp_path, path) {
                Ok(()) => std::fs::remove_file(&rollback_path)
                    .map_err(|error| format!("replacement installed but cleanup failed: {error}")),
                Err(replace_error) => {
                    let restore_result = std::fs::rename(&rollback_path, path);
                    match restore_result {
                        Ok(()) => Err(format!(
                            "replacement failed: {replace_error}; original cache was restored; initial error: {first_error}"
                        )),
                        Err(restore_error) => Err(format!(
                            "replacement failed: {replace_error}; original cache restore failed: {restore_error}; initial error: {first_error}"
                        )),
                    }
                }
            }
        }
        Err(error) => Err(error.to_string()),
    }
}

fn unique_cache_sidecar_path(path: &std::path::Path, marker: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = JOURNAL_CACHE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut file_name = path
        .file_name()
        .map(std::ffi::OsString::from)
        .unwrap_or_else(|| std::ffi::OsString::from("journal_cache.json"));
    file_name.push(format!(
        ".{marker}.{}.{}.{}",
        std::process::id(),
        nanos,
        counter
    ));
    path.parent()
        .map(|parent| parent.join(&file_name))
        .unwrap_or_else(|| std::path::PathBuf::from(file_name))
}

#[cfg(test)]
mod tests;
