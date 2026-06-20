use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static IN_MEMORY_CONFIG_MODE: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_in_memory_config_mode(enabled: bool) {
    IN_MEMORY_CONFIG_MODE.store(enabled, Ordering::Relaxed);
}

pub(crate) fn in_memory_config_mode() -> bool {
    IN_MEMORY_CONFIG_MODE.load(Ordering::Relaxed)
}

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
    if in_memory_config_mode() {
        return None;
    }
    dirs::config_dir().map(|d| d.join("kerosene").join("config.json"))
}

pub(in crate::config) fn backup_config_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

pub(crate) fn user_config_dir() -> &'static str {
    "<config-dir>"
}

pub(crate) fn user_config_path(path: &Path) -> String {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return user_config_dir().to_string();
    };

    if file_name == "config.json" || file_name == "config.json.bak" {
        return file_name.to_string();
    }
    if file_name.starts_with("config.json.") || file_name.starts_with("config.json.bak.") {
        return format!("{}/{file_name}", user_config_dir());
    }
    if file_name == "fonts" || file_name == "sounds" {
        return format!("{}/{file_name}", user_config_dir());
    }
    if file_name == "cache" {
        return format!("{}/{file_name}", user_config_dir());
    }
    if file_name.starts_with("journal_cache_") {
        return format!("{}/journal_cache_<redacted>.json", user_config_dir());
    }

    format!("{}/...", user_config_dir())
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
    if in_memory_config_mode() {
        return None;
    }
    dirs::config_dir().map(|d| {
        d.join("kerosene")
            .join(format!("journal_cache_{}.json", address))
    })
}

#[cfg(test)]
pub fn api_cache_dir() -> Option<PathBuf> {
    // Tests must never read or write the real platform cache directory; the
    // cache's own unit tests exercise the *_from_dir helpers with temp roots.
    None
}

#[cfg(not(test))]
pub fn api_cache_dir() -> Option<PathBuf> {
    if in_memory_config_mode() {
        return None;
    }
    dirs::config_dir().map(|d| d.join("kerosene").join("cache").join("v1"))
}

pub fn font_storage_dir() -> Option<PathBuf> {
    if in_memory_config_mode() {
        return None;
    }
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
    if in_memory_config_mode() {
        return None;
    }
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

#[cfg(test)]
mod tests {
    use super::user_config_path;
    use std::path::Path;

    #[test]
    fn user_config_path_redacts_directory_components() {
        assert_eq!(
            user_config_path(Path::new("/home/alice/.config/kerosene/config.json")),
            "config.json"
        );
        assert_eq!(
            user_config_path(Path::new("/home/alice/.config/kerosene/config.json.bak")),
            "config.json.bak"
        );
        assert_eq!(
            user_config_path(Path::new(
                "/home/alice/.config/kerosene/config.json.tmp-123"
            )),
            "<config-dir>/config.json.tmp-123"
        );
        assert_eq!(
            user_config_path(Path::new("/home/alice/.config/kerosene/fonts")),
            "<config-dir>/fonts"
        );
        assert_eq!(
            user_config_path(Path::new("/home/alice/.config/kerosene/cache")),
            "<config-dir>/cache"
        );
        assert_eq!(
            user_config_path(Path::new(
                "/home/alice/.config/kerosene/journal_cache_0xabc.json"
            )),
            "<config-dir>/journal_cache_<redacted>.json"
        );
    }
}
