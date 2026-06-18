#[cfg(unix)]
use super::super::files::write_with_restricted_permissions;
use super::super::files::{backup_config_path, load_config_from_path, save_config_to_path};
use super::super::{AddressBookEntryConfig, KeroseneConfig, take_config_warnings};
use super::unique_test_config_path;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

mod persistence;
#[cfg(unix)]
mod unix_permissions;

fn test_path(name: &str) -> PathBuf {
    unique_test_config_path(name)
}

fn address_book_config(
    address: &str,
    label: &str,
    color: Option<&str>,
    tags: &[&str],
) -> KeroseneConfig {
    KeroseneConfig {
        address_book: vec![AddressBookEntryConfig {
            address: address.to_string(),
            label: label.to_string(),
            color: color.map(str::to_string),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
        }],
        ..Default::default()
    }
}

fn cleanup_path(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

fn create_parent_dir(path: &Path) {
    let Some(parent) = path.parent() else {
        panic!("test path should have a parent: {}", path.display());
    };
    if let Err(e) = std::fs::create_dir_all(parent) {
        panic!("create test directory {} failed: {e}", parent.display());
    }
}

fn config_warning_guard() -> MutexGuard<'static, ()> {
    static WARNING_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = match WARNING_TEST_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let _ = take_config_warnings();
    guard
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    if let Err(e) = std::fs::write(path, contents) {
        panic!("write {} failed: {e}", path.display());
    }
}

fn config_json(config: &KeroseneConfig) -> String {
    match serde_json::to_string_pretty(config) {
        Ok(json) => json,
        Err(e) => panic!("serialize test config failed: {e}"),
    }
}

fn save_config_fixture(path: &Path, config: &KeroseneConfig) {
    if let Err(e) = save_config_to_path(path, config) {
        panic!("save config {} failed: {e}", path.display());
    }
}

fn load_existing_config(path: &Path) -> KeroseneConfig {
    match load_config_from_path(path) {
        Ok(Some(config)) => config,
        Ok(None) => panic!("config should exist at {}", path.display()),
        Err(e) => panic!("load config {} failed: {e}", path.display()),
    }
}

#[cfg(unix)]
fn set_file_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;

    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)) {
        panic!("set mode {:o} on {} failed: {e}", mode, path.display());
    }
}

#[cfg(unix)]
fn file_mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;

    match std::fs::metadata(path) {
        Ok(metadata) => metadata.permissions().mode(),
        Err(e) => panic!("read metadata for {} failed: {e}", path.display()),
    }
}

#[cfg(unix)]
fn read_file(path: &Path) -> Vec<u8> {
    match std::fs::read(path) {
        Ok(contents) => contents,
        Err(e) => panic!("read {} failed: {e}", path.display()),
    }
}

#[cfg(unix)]
fn write_restricted(path: &Path, contents: &[u8]) {
    if let Err(e) = write_with_restricted_permissions(path, contents) {
        panic!("restricted write {} failed: {e}", path.display());
    }
}

#[cfg(unix)]
fn assert_owner_only_mode(path: &Path, label: &str) {
    let mode = file_mode(path);
    assert_eq!(
        mode & 0o777,
        0o600,
        "{label}: expected 0o600, got {:o}",
        mode & 0o777
    );
}
