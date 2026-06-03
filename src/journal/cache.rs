use super::normalize_fills;
use crate::api::UserFill;
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

    // On Unix `rename` atomically replaces the destination. Do not delete the
    // existing cache on failure: a failed save should leave the last good cache
    // available for recovery.
    if let Err(rename_err) = std::fs::rename(&temp_path, &path) {
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
                            parent.display()
                        )),
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!(
                "read journal cache directory {} failed: {e}",
                parent.display()
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
        Err(e) => Err(format!("remove {} failed: {e}", path.display())),
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

#[cfg(test)]
mod tests;
