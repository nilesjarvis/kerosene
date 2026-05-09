use super::normalize_fills;
use crate::api::UserFill;

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
    let temp_path = path.with_extension("json.tmp");
    std::fs::write(&temp_path, &json)
        .map_err(|e| format!("Failed to write temporary cache file: {e}"))?;

    match std::fs::rename(&temp_path, &path) {
        Ok(()) => {}
        Err(rename_err) => {
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("Failed to replace cache file: {e}"))?;
                std::fs::rename(&temp_path, &path)
                    .map_err(|e| format!("Failed to move cache file into place: {e}"))?;
            } else {
                return Err(format!(
                    "Failed to move cache file into place: {rename_err}"
                ));
            }
        }
    }

    Ok(())
}
