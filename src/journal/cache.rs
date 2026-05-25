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
