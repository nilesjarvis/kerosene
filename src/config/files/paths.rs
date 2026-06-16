use std::ffi::OsString;
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

fn unique_config_sidecar_path(path: &Path, marker: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut file_name = path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("config.json"));
    file_name.push(format!(".{marker}-{pid}-{nanos}"));

    path.parent()
        .map(|parent| parent.join(&file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

pub(in crate::config) fn config_sidecar_prefix(path: &Path, marker: &str) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name}.{marker}-"))
}

pub(super) fn temp_config_path(path: &Path) -> PathBuf {
    unique_config_sidecar_path(path, "tmp")
}

pub(super) fn replacement_rollback_path(path: &Path) -> PathBuf {
    unique_config_sidecar_path(path, "replace-old")
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
    if !safe_stored_file_name(file_name) {
        return None;
    }

    font_storage_dir().map(|dir| dir.join(file_name))
}

pub fn sound_storage_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("kerosene").join("sounds"))
}

pub fn custom_sound_path(file_name: &str) -> Option<PathBuf> {
    let file_name = file_name.trim();
    if !safe_stored_file_name(file_name) {
        return None;
    }

    sound_storage_dir().map(|dir| dir.join(file_name))
}

fn safe_stored_file_name(file_name: &str) -> bool {
    !file_name.is_empty()
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && !file_name.contains("..")
}
