use super::files::{user_config_dir, user_config_path};
use super::{
    AccountProfile, backup_config_path, clear_all_keychain_secrets, config_path,
    config_sidecar_prefix,
};
use crate::telegram_fast_feed::clear_telegram_fast_session_files_at;
use std::path::Path;

// ---------------------------------------------------------------------------
// Config Clearing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearConfigSummary {
    pub files_removed: usize,
    pub file_cleanup_failed: bool,
    pub keychain_entries_cleared: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfigFileCleanupSummary {
    pub(super) files_removed: usize,
    pub(super) file_cleanup_failed: bool,
    pub(super) warnings: Vec<String>,
}

impl ConfigFileCleanupSummary {
    fn success(files_removed: usize) -> Self {
        Self {
            files_removed,
            file_cleanup_failed: false,
            warnings: Vec::new(),
        }
    }

    fn config_file_failure(error: String) -> Self {
        Self {
            files_removed: 0,
            file_cleanup_failed: true,
            warnings: vec![format!("config file cleanup failed: {error}")],
        }
    }

    fn add_removed(&mut self, count: usize) {
        self.files_removed += count;
    }

    fn add_warning(&mut self, message: String) {
        self.warnings.push(message);
    }

    fn add_blocking_warning(&mut self, message: String) {
        self.file_cleanup_failed = true;
        self.add_warning(message);
    }

    #[cfg(test)]
    fn extend(&mut self, other: Self) {
        self.files_removed += other.files_removed;
        self.file_cleanup_failed |= other.file_cleanup_failed;
        self.warnings.extend(other.warnings);
    }
}

fn remove_file_if_exists(path: &Path) -> Result<bool, String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("remove {} failed: {e}", user_config_path(path))),
    }
}

fn remove_path_tree_if_exists(path: &Path) -> Result<bool, String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("inspect {} failed: {e}", user_config_path(path))),
    };

    let result = if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };

    result
        .map(|()| true)
        .map_err(|e| format!("remove {} failed: {e}", user_config_path(path)))
}

pub(super) fn clear_config_path_family(path: &Path) -> Result<usize, String> {
    let mut removed = 0;
    let mut errors = Vec::new();
    let backup_path = backup_config_path(path);

    for candidate in [path.to_path_buf(), backup_path.clone()] {
        match remove_file_if_exists(&candidate) {
            Ok(true) => removed += 1,
            Ok(false) => {}
            Err(e) => errors.push(e),
        }
    }

    if let Some(parent) = path.parent() {
        let sidecar_prefixes: Vec<String> = [path, backup_path.as_path()]
            .into_iter()
            .flat_map(|candidate| {
                ["tmp", "replace-old"]
                    .into_iter()
                    .filter_map(move |marker| config_sidecar_prefix(candidate, marker))
            })
            .collect();
        match std::fs::read_dir(parent) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let sidecar_path = entry.path();
                            let is_config_sidecar = sidecar_path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .is_some_and(|name| {
                                    sidecar_prefixes
                                        .iter()
                                        .any(|prefix| name.starts_with(prefix))
                                });

                            if is_config_sidecar {
                                match remove_file_if_exists(&sidecar_path) {
                                    Ok(true) => removed += 1,
                                    Ok(false) => {}
                                    Err(e) => errors.push(e),
                                }
                            }
                        }
                        Err(e) => errors.push(format!(
                            "read config directory {} entry failed: {e}",
                            user_config_dir()
                        )),
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!(
                "read config directory {} failed: {e}",
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

fn clear_config_asset_dirs(parent: &Path) -> Result<usize, String> {
    let mut removed = 0;
    let mut errors = Vec::new();

    for candidate in [parent.join("fonts"), parent.join("sounds")] {
        match remove_path_tree_if_exists(&candidate) {
            Ok(true) => removed += 1,
            Ok(false) => {}
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(removed)
    } else {
        Err(errors.join("; "))
    }
}

fn clear_journal_cache_files(parent: &Path) -> Result<usize, String> {
    let mut removed = 0;
    let mut errors = Vec::new();

    match std::fs::read_dir(parent) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        let is_journal_cache = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| {
                                name.starts_with("journal_cache_")
                                    && (name.ends_with(".json") || name.contains(".json.tmp."))
                            });
                        if is_journal_cache {
                            match remove_file_if_exists(&path) {
                                Ok(true) => removed += 1,
                                Ok(false) => {}
                                Err(e) => errors.push(e),
                            }
                        }
                    }
                    Err(e) => errors.push(format!(
                        "read config directory {} entry failed: {e}",
                        user_config_dir()
                    )),
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => errors.push(format!(
            "read config directory {} failed: {e}",
            user_config_dir()
        )),
    }

    if errors.is_empty() {
        Ok(removed)
    } else {
        Err(errors.join("; "))
    }
}

pub(super) fn clear_config_side_files_at(parent: &Path) -> ConfigFileCleanupSummary {
    let mut summary = ConfigFileCleanupSummary::success(0);

    let telegram_session_path = parent.join("telegram_fast.session");
    match clear_telegram_fast_session_files_at(&telegram_session_path) {
        Ok(count) => summary.add_removed(count),
        Err(e) => summary.add_blocking_warning(format!("Telegram session cleanup failed: {e}")),
    }
    match clear_journal_cache_files(parent) {
        Ok(count) => summary.add_removed(count),
        Err(e) => summary.add_blocking_warning(format!("journal cache cleanup failed: {e}")),
    }
    match clear_config_asset_dirs(parent) {
        Ok(count) => summary.add_removed(count),
        Err(e) => summary.add_warning(format!("asset cleanup failed: {e}")),
    }

    summary
}

#[cfg(test)]
pub(super) fn clear_config_files_at(path: &Path) -> ConfigFileCleanupSummary {
    let mut summary = ConfigFileCleanupSummary::success(0);

    match clear_config_path_family(path) {
        Ok(count) => summary.add_removed(count),
        Err(e) => {
            summary.add_blocking_warning(format!("config file cleanup failed: {e}"));
            return summary;
        }
    }

    if let Some(parent) = path.parent() {
        summary.extend(clear_config_side_files_at(parent));
    }

    summary
}

fn keychain_entry_count(profiles: &[AccountProfile]) -> usize {
    profiles
        .iter()
        .filter(|profile| !profile.secret_id.trim().is_empty())
        .count()
        * 2
        + 4
}

pub(super) fn clear_all_configs_with<K, P, S>(
    profiles: &[AccountProfile],
    clear_keychain: K,
    clear_primary_files: P,
    clear_side_files: S,
) -> ClearConfigSummary
where
    K: FnOnce(&[AccountProfile]) -> Result<(), String>,
    P: FnOnce() -> ConfigFileCleanupSummary,
    S: FnOnce() -> ConfigFileCleanupSummary,
{
    let mut files_removed = 0;
    let mut keychain_entries_cleared = 0;
    let mut warnings = Vec::new();

    match clear_keychain(profiles) {
        Ok(()) => {
            keychain_entries_cleared = keychain_entry_count(profiles);
        }
        Err(e) => {
            warnings.push(format!("keychain cleanup failed: {e}"));
            return ClearConfigSummary {
                files_removed,
                file_cleanup_failed: true,
                keychain_entries_cleared,
                warnings,
            };
        }
    }

    let primary_file_summary = clear_primary_files();
    files_removed += primary_file_summary.files_removed;
    let primary_file_cleanup_failed = primary_file_summary.file_cleanup_failed;
    warnings.extend(primary_file_summary.warnings);
    if primary_file_cleanup_failed {
        return ClearConfigSummary {
            files_removed,
            file_cleanup_failed: true,
            keychain_entries_cleared,
            warnings,
        };
    }

    let side_file_summary = clear_side_files();
    files_removed += side_file_summary.files_removed;
    warnings.extend(side_file_summary.warnings);
    if side_file_summary.file_cleanup_failed {
        return ClearConfigSummary {
            files_removed,
            file_cleanup_failed: true,
            keychain_entries_cleared,
            warnings,
        };
    }

    ClearConfigSummary {
        files_removed,
        file_cleanup_failed: false,
        keychain_entries_cleared,
        warnings,
    }
}

pub fn clear_all_configs(profiles: &[AccountProfile]) -> Result<ClearConfigSummary, String> {
    let Some(path) = config_path() else {
        return Ok(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: true,
            keychain_entries_cleared: 0,
            warnings: vec![
                "config file cleanup failed: platform config directory is unavailable".to_string(),
            ],
        });
    };
    let parent = path.parent().map(Path::to_path_buf);

    Ok(clear_all_configs_with(
        profiles,
        clear_all_keychain_secrets,
        || match clear_config_path_family(&path) {
            Ok(count) => ConfigFileCleanupSummary::success(count),
            Err(e) => ConfigFileCleanupSummary::config_file_failure(e),
        },
        || {
            parent
                .as_deref()
                .map(clear_config_side_files_at)
                .unwrap_or_else(|| ConfigFileCleanupSummary::success(0))
        },
    ))
}
