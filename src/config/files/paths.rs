use std::path::{Path, PathBuf};

/// Platform-appropriate config file path.
/// Linux: ~/.config/kerosene/config.json
/// macOS: ~/Library/Application Support/kerosene/config.json
/// Windows: %APPDATA%\kerosene\config.json
#[cfg(test)]
pub(in crate::config) fn config_path() -> Option<PathBuf> {
    None
}

#[cfg(not(test))]
pub(in crate::config) fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("kerosene").join("config.json"))
}

pub(in crate::config) fn backup_config_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

pub(super) fn temp_config_path(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.with_extension(format!("json.tmp-{pid}-{nanos}"))
}

pub fn journal_cache_path(address: &str) -> Option<PathBuf> {
    dirs::config_dir().map(|d| {
        d.join("kerosene")
            .join(format!("journal_cache_{}.json", address))
    })
}

pub fn font_storage_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("kerosene").join("fonts"))
}

pub fn custom_font_path(file_name: &str) -> Option<PathBuf> {
    let file_name = file_name.trim();
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains("..")
    {
        return None;
    }

    font_storage_dir().map(|dir| dir.join(file_name))
}
