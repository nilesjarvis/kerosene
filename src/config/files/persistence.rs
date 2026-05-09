use crate::config::{KeroseneConfig, push_config_warning};
use std::path::Path;

use super::paths::{backup_config_path, temp_config_path};

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
    if temp_path.exists() {
        std::fs::remove_file(&temp_path).map_err(|e| {
            format!(
                "remove stale temp config {} failed: {e}",
                temp_path.display()
            )
        })?;
    }

    std::fs::write(&temp_path, json.as_bytes())
        .map_err(|e| format!("write temp config {} failed: {e}", temp_path.display()))?;

    if let Ok(file) = std::fs::File::open(&temp_path) {
        let _ = file.sync_all();
    }

    if matches!(read_config_from_path(path), Ok(Some(_))) {
        let backup_path = backup_config_path(path);
        std::fs::copy(path, &backup_path).map_err(|e| {
            format!(
                "backup config {} to {} failed: {e}",
                path.display(),
                backup_path.display()
            )
        })?;
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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("set config permissions {} failed: {e}", path.display()))?;
    }

    Ok(())
}
