use crate::config::{KeroseneConfig, push_config_warning};
use std::io::Write;
use std::path::Path;

use super::paths::{backup_config_path, temp_config_path};

/// Create `path` with `contents` and owner-only permissions.
///
/// The file is opened with exclusive creation so callers never truncate or
/// rewrite a pre-existing world-readable file while sensitive config bytes are
/// being written. On Unix the mode is applied at creation time, before the
/// first byte is written; on non-Unix platforms this falls back to the previous
/// behaviour (`fs::write` using default permissions) because Windows ACL
/// hardening is a separate problem.
pub(in crate::config) fn write_with_restricted_permissions(
    path: &Path,
    contents: &[u8],
) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| format!("create {} failed: {e}", path.display()))?;
        file.write_all(contents)
            .map_err(|e| format!("write {} failed: {e}", path.display()))?;
        file.sync_all()
            .map_err(|e| format!("sync {} failed: {e}", path.display()))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents).map_err(|e| format!("write {} failed: {e}", path.display()))
    }
}

#[cfg(unix)]
fn set_restricted_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("set permissions {} failed: {e}", path.display()))
}

fn replace_with_restricted_permissions(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temp_path = temp_config_path(path);
    write_with_restricted_permissions(&temp_path, contents)?;

    match std::fs::rename(&temp_path, path) {
        Ok(()) => {}
        Err(rename_error) if path.exists() => {
            std::fs::remove_file(path).map_err(|e| {
                format!(
                    "replace {} failed: {rename_error}; remove existing failed: {e}",
                    path.display()
                )
            })?;
            std::fs::rename(&temp_path, path).map_err(|e| {
                format!(
                    "replace {} failed after removing existing file: {e}",
                    path.display()
                )
            })?;
        }
        Err(e) => {
            return Err(format!(
                "rename restricted temp config to {} failed: {e}",
                path.display()
            ));
        }
    }

    #[cfg(unix)]
    set_restricted_permissions(path)?;

    Ok(())
}

fn read_config_from_path(path: &Path) -> Result<Option<KeroseneConfig>, String> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read {} failed: {e}", path.display())),
    };

    serde_json::from_str(&contents)
        .map(Some)
        .map_err(|e| format!("parse {} failed: {e}", path.display()))
}

pub(in crate::config) fn load_config_from_path(
    path: &Path,
) -> Result<Option<KeroseneConfig>, String> {
    match read_config_from_path(path) {
        Ok(config) => Ok(config),
        Err(primary_error) => {
            let backup_path = backup_config_path(path);
            match read_config_from_path(&backup_path) {
                Ok(Some(config)) => {
                    push_config_warning(format!(
                        "Loaded backup config because primary config failed: {primary_error}"
                    ));
                    Ok(Some(config))
                }
                Ok(None) => Err(primary_error),
                Err(backup_error) => Err(format!("{primary_error}; backup failed: {backup_error}")),
            }
        }
    }
}

pub(in crate::config) fn save_config_to_path(
    path: &Path,
    config: &KeroseneConfig,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create config directory {} failed: {e}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("serialize config failed: {e}"))?;
    let temp_path = temp_config_path(path);
    write_with_restricted_permissions(&temp_path, json.as_bytes())?;

    if let Ok(Some(previous_config)) = read_config_from_path(path) {
        let backup_path = backup_config_path(path);
        let backup_json = serde_json::to_string_pretty(&previous_config)
            .map_err(|e| format!("sanitize backup config failed: {e}"))?;
        replace_with_restricted_permissions(&backup_path, backup_json.as_bytes())?;
    }

    match std::fs::rename(&temp_path, path) {
        Ok(()) => {}
        Err(rename_error) if path.exists() => {
            std::fs::remove_file(path).map_err(|e| {
                format!(
                    "replace config {} failed: {rename_error}; remove existing failed: {e}",
                    path.display()
                )
            })?;
            std::fs::rename(&temp_path, path).map_err(|e| {
                format!(
                    "replace config {} failed after removing existing file: {e}",
                    path.display()
                )
            })?;
        }
        Err(e) => {
            return Err(format!(
                "rename temp config to {} failed: {e}",
                path.display()
            ));
        }
    }

    // The rename above carries the temp file's 0o600 onto `path`, so this
    // is belt-and-braces in case the prior on-disk config had looser
    // permissions for some reason.
    #[cfg(unix)]
    set_restricted_permissions(path)?;

    Ok(())
}
