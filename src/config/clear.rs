use super::{AccountProfile, backup_config_path, config_path};
use super::{clear_global_secrets, clear_profile_secrets};
use std::path::Path;

// ---------------------------------------------------------------------------
// Config Clearing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearConfigSummary {
    pub files_removed: usize,
    pub keychain_entries_cleared: usize,
    pub warnings: Vec<String>,
}

fn remove_file_if_exists(path: &Path) -> Result<bool, String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("remove {} failed: {e}", path.display())),
    }
}

pub(super) fn clear_config_path_family(path: &Path) -> Result<usize, String> {
    let mut removed = 0;
    let mut errors = Vec::new();

    for candidate in [path.to_path_buf(), backup_config_path(path)] {
        match remove_file_if_exists(&candidate) {
            Ok(true) => removed += 1,
            Ok(false) => {}
            Err(e) => errors.push(e),
        }
    }

    if let (Some(parent), Some(file_name)) = (
        path.parent(),
        path.file_name().and_then(|name| name.to_str()),
    ) {
        let temp_prefix = format!("{file_name}.tmp-");
        match std::fs::read_dir(parent) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let temp_path = entry.path();
                            let is_temp_config = temp_path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .is_some_and(|name| name.starts_with(&temp_prefix));

                            if is_temp_config {
                                match remove_file_if_exists(&temp_path) {
                                    Ok(true) => removed += 1,
                                    Ok(false) => {}
                                    Err(e) => errors.push(e),
                                }
                            }
                        }
                        Err(e) => errors.push(format!(
                            "read config directory {} entry failed: {e}",
                            parent.display()
                        )),
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!(
                "read config directory {} failed: {e}",
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

pub fn clear_config_files() -> Result<usize, String> {
    let Some(path) = config_path() else {
        return Err("platform config directory is unavailable".to_string());
    };

    let mut removed = 0;
    let mut errors = Vec::new();

    match clear_config_path_family(&path) {
        Ok(count) => removed += count,
        Err(e) => errors.push(e),
    }

    if errors.is_empty() {
        Ok(removed)
    } else {
        Err(errors.join("; "))
    }
}

pub fn clear_all_configs(profiles: &[AccountProfile]) -> Result<ClearConfigSummary, String> {
    let files_removed = clear_config_files()?;

    let mut keychain_entries_cleared = 0;
    let mut warnings = Vec::new();
    for profile in profiles {
        if profile.secret_id.trim().is_empty() {
            continue;
        }

        match clear_profile_secrets(profile) {
            Ok(()) => keychain_entries_cleared += 2,
            Err(e) => warnings.push(format!("{} keychain cleanup skipped: {e}", profile.name)),
        }
    }

    match clear_global_secrets() {
        Ok(()) => keychain_entries_cleared += 2,
        Err(e) => warnings.push(format!("global keychain cleanup skipped: {e}")),
    }

    Ok(ClearConfigSummary {
        files_removed,
        keychain_entries_cleared,
        warnings,
    })
}
