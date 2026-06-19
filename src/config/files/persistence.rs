use crate::config::{KeroseneConfig, push_config_warning};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use zeroize::Zeroizing;

use super::{
    normalization,
    paths::{
        backup_config_path, config_sidecar_prefix, replacement_rollback_path, temp_config_path,
        user_config_dir, user_config_path,
    },
};

const INSTALLED_SNAPSHOT_ERROR_PREFIX: &str = "config snapshot was installed, but ";

pub(crate) fn config_save_installed_snapshot(error: &str) -> bool {
    error.starts_with(INSTALLED_SNAPSHOT_ERROR_PREFIX)
}

fn installed_snapshot_error(error: String) -> String {
    format!("{INSTALLED_SNAPSHOT_ERROR_PREFIX}{error}")
}

#[cfg(test)]
pub(crate) fn installed_config_save_error_for_test(error: &str) -> String {
    installed_snapshot_error(error.to_string())
}

/// Create `path` with `contents` and owner-only permissions.
///
/// The file is opened with exclusive creation so callers never truncate or
/// rewrite a pre-existing world-readable file while sensitive config bytes are
/// being written. On Unix the mode is applied at creation time, before the
/// first byte is written; on non-Unix platforms this still uses exclusive
/// creation and syncs the file, but Windows ACL hardening is a separate
/// problem.
pub(in crate::config) fn write_with_restricted_permissions(
    path: &Path,
    contents: &[u8],
) -> Result<(), String> {
    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| format!("create {} failed: {e}", user_config_path(path)))?
    };
    #[cfg(not(unix))]
    let mut file = {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("create {} failed: {e}", user_config_path(path)))?
    };

    if let Err(e) = file.write_all(contents) {
        drop(file);
        cleanup_file_best_effort(path);
        return Err(format!("write {} failed: {e}", user_config_path(path)));
    }

    if let Err(e) = file.sync_all() {
        drop(file);
        cleanup_file_best_effort(path);
        return Err(format!("sync {} failed: {e}", user_config_path(path)));
    }

    Ok(())
}

#[cfg(unix)]
fn set_restricted_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("set permissions {} failed: {e}", user_config_path(path)))
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let dir = std::fs::File::open(parent)
        .map_err(|e| format!("open config directory {} failed: {e}", user_config_dir()))?;
    dir.sync_all()
        .map_err(|e| format!("sync config directory {} failed: {e}", user_config_dir()))
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn cleanup_file_best_effort(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn remove_file_if_present(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn remove_missing_primary_save_sidecars(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let backup_path = backup_config_path(path);
    let prefixes: Vec<String> = [path, backup_path.as_path()]
        .into_iter()
        .flat_map(|candidate| {
            ["tmp", "replace-old"]
                .into_iter()
                .filter_map(move |marker| config_sidecar_prefix(candidate, marker))
        })
        .collect();
    if prefixes.is_empty() {
        return Ok(());
    }

    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "scan stale config sidecars in {} failed: {error}",
                user_config_dir()
            ));
        }
    };

    let mut errors = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(format!(
                    "read stale config sidecar entry in {} failed: {error}",
                    user_config_dir()
                ));
                continue;
            }
        };
        let sidecar_path = entry.path();
        let is_stale_sidecar = sidecar_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| prefixes.iter().any(|prefix| name.starts_with(prefix)));
        if !is_stale_sidecar {
            continue;
        }
        if let Err(error) = remove_file_if_present(&sidecar_path) {
            errors.push(format!(
                "remove {} failed: {error}",
                user_config_path(&sidecar_path)
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn remove_missing_primary_save_artifacts(path: &Path) -> Result<(), String> {
    let backup_path = backup_config_path(path);
    remove_file_if_present(&backup_path).map_err(|e| {
        format!(
            "remove stale backup config {} failed: {e}",
            user_config_path(&backup_path)
        )
    })?;
    remove_missing_primary_save_sidecars(path)?;
    sync_parent_directory(path)
}

trait ReplaceFileOps {
    fn rename(&mut self, from: &Path, to: &Path) -> Result<(), String>;
    fn exists(&mut self, path: &Path) -> bool;
    fn replaceable_existing(&mut self, path: &Path) -> bool;
    fn cleanup_rollback(&mut self, path: &Path) -> Result<(), String>;
    fn sync_parent(&mut self, path: &Path) -> Result<(), String>;
}

#[derive(Debug)]
struct ReplaceFileError {
    message: String,
    installed_replacement: bool,
}

impl ReplaceFileError {
    fn before_install(message: String) -> Self {
        Self {
            message,
            installed_replacement: false,
        }
    }

    fn after_install(message: String) -> Self {
        Self {
            message,
            installed_replacement: true,
        }
    }
}

impl std::fmt::Display for ReplaceFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

struct StdReplaceFileOps;

impl ReplaceFileOps for StdReplaceFileOps {
    fn rename(&mut self, from: &Path, to: &Path) -> Result<(), String> {
        std::fs::rename(from, to).map_err(|e| e.to_string())
    }

    fn exists(&mut self, path: &Path) -> bool {
        path.exists()
    }

    fn replaceable_existing(&mut self, path: &Path) -> bool {
        path.symlink_metadata()
            .map(|metadata| {
                let file_type = metadata.file_type();
                file_type.is_file() || file_type.is_symlink()
            })
            .unwrap_or(false)
    }

    fn cleanup_rollback(&mut self, path: &Path) -> Result<(), String> {
        remove_file_if_present(path)
    }

    fn sync_parent(&mut self, path: &Path) -> Result<(), String> {
        sync_parent_directory(path)
    }
}

fn replace_temp_file_with(
    temp_path: &Path,
    path: &Path,
    context: &str,
    mut ops: impl ReplaceFileOps,
) -> Result<(), ReplaceFileError> {
    match ops.rename(temp_path, path) {
        Ok(()) => {
            ops.sync_parent(path).map_err(|error| {
                ReplaceFileError::after_install(format!(
                    "{context} {} replaced but sync after install failed: {error}",
                    user_config_path(path)
                ))
            })?;
            Ok(())
        }
        Err(rename_error) if ops.exists(path) && ops.replaceable_existing(path) => {
            let rollback_path = replacement_rollback_path(path);
            ops.rename(path, &rollback_path).map_err(|e| {
                ReplaceFileError::before_install(format!(
                    "{context} {} failed: {rename_error}; staging existing file for rollback failed: {e}",
                    user_config_path(path)
                ))
            })?;

            match ops.rename(temp_path, path) {
                Ok(()) => {
                    let install_sync_result = ops.sync_parent(path);
                    let cleanup_result = ops.cleanup_rollback(&rollback_path);
                    match (install_sync_result, cleanup_result) {
                        (Ok(()), Ok(())) => {}
                        (Err(sync_error), Ok(())) => {
                            return Err(ReplaceFileError::after_install(format!(
                                "{context} {} replaced but sync after install failed: {sync_error}; rollback was cleaned up",
                                user_config_path(path)
                            )));
                        }
                        (Ok(()), Err(cleanup_error)) => {
                            return Err(ReplaceFileError::after_install(format!(
                                "{context} {} replaced but cleanup rollback {} failed: {cleanup_error}",
                                user_config_path(path),
                                user_config_path(&rollback_path)
                            )));
                        }
                        (Err(sync_error), Err(cleanup_error)) => {
                            return Err(ReplaceFileError::after_install(format!(
                                "{context} {} replaced but sync after install failed: {sync_error}; cleanup rollback {} failed: {cleanup_error}",
                                user_config_path(path),
                                user_config_path(&rollback_path)
                            )));
                        }
                    }
                    ops.sync_parent(path).map_err(|error| {
                        ReplaceFileError::after_install(format!(
                            "{context} {} replaced but final sync failed: {error}",
                            user_config_path(path)
                        ))
                    })?;
                    Ok(())
                }
                Err(replace_error) => {
                    let restore_result = ops.rename(&rollback_path, path);
                    match restore_result {
                        Ok(()) => Err(ReplaceFileError::before_install(format!(
                            "{context} {} failed after staging existing file for rollback: {replace_error}; original file was restored",
                            user_config_path(path)
                        ))),
                        Err(restore_error) => Err(ReplaceFileError::before_install(format!(
                            "{context} {} failed after staging existing file for rollback: {replace_error}; restore original failed: {restore_error}",
                            user_config_path(path)
                        ))),
                    }
                }
            }
        }
        Err(e) => Err(ReplaceFileError::before_install(format!(
            "{context} {} failed: {e}",
            user_config_path(path)
        ))),
    }
}

fn replace_temp_file(temp_path: &Path, path: &Path, context: &str) -> Result<(), ReplaceFileError> {
    replace_temp_file_with(temp_path, path, context, StdReplaceFileOps)
}

fn replace_with_restricted_permissions(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temp_path = temp_config_path(path);
    write_with_restricted_permissions(&temp_path, contents)?;
    if let Err(e) = replace_temp_file(&temp_path, path, "replace restricted config") {
        cleanup_file_best_effort(&temp_path);
        return Err(e.to_string());
    }

    #[cfg(unix)]
    set_restricted_permissions(path)?;

    Ok(())
}

fn read_config_from_path(path: &Path) -> Result<Option<KeroseneConfig>, String> {
    let contents = Zeroizing::new(match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read {} failed: {e}", user_config_path(path))),
    });

    serde_json::from_str(contents.as_str())
        .map(Some)
        .map_err(|e| format!("parse {} failed: {e}", user_config_path(path)))
}

fn backup_config_for_save(
    mut previous_config: KeroseneConfig,
    config: &KeroseneConfig,
) -> KeroseneConfig {
    let cleanup_intent_changed = previous_config.pending_keychain_profile_deletions
        != config.pending_keychain_profile_deletions
        || previous_config.pending_keychain_cleanup_all != config.pending_keychain_cleanup_all;
    let credentials_changed = previous_config.credential_storage_mode
        != config.credential_storage_mode
        || previous_config.encrypted_secrets != config.encrypted_secrets
        || cleanup_intent_changed;

    if cleanup_intent_changed {
        previous_config.pending_keychain_profile_deletions =
            config.pending_keychain_profile_deletions.clone();
        previous_config.pending_keychain_cleanup_all = config.pending_keychain_cleanup_all;
        normalization::apply_pending_keychain_profile_deletions(&mut previous_config);
    }
    if credentials_changed {
        prune_backup_accounts_absent_from_current(&mut previous_config, config);
    }

    if previous_config.credential_storage_mode != config.credential_storage_mode
        || previous_config.encrypted_secrets != config.encrypted_secrets
    {
        previous_config.credential_storage_mode = config.credential_storage_mode;
        previous_config.encrypted_secrets = config.encrypted_secrets.clone();
    }

    previous_config
}

fn prune_backup_accounts_absent_from_current(
    previous_config: &mut KeroseneConfig,
    config: &KeroseneConfig,
) {
    if previous_config.accounts.is_empty() {
        return;
    }

    let current_ids: HashSet<&str> = config
        .accounts
        .iter()
        .map(|profile| profile.secret_id.as_str())
        .collect();
    let active_index = previous_config.active_account_index;
    let active_removed = previous_config
        .accounts
        .get(active_index)
        .is_some_and(|profile| !current_ids.contains(profile.secret_id.as_str()));
    let removed_before_active = previous_config
        .accounts
        .iter()
        .enumerate()
        .filter(|(index, profile)| {
            *index < active_index && !current_ids.contains(profile.secret_id.as_str())
        })
        .count();

    previous_config
        .accounts
        .retain(|profile| current_ids.contains(profile.secret_id.as_str()));
    previous_config
        .hidden_positions_by_account
        .retain(|account_key, _| current_ids.contains(account_key.as_str()));
    previous_config
        .journal_entries_by_account
        .retain(|account_key, _| current_ids.contains(account_key.as_str()));

    if active_removed {
        previous_config.active_account_index = 0;
    } else {
        previous_config.active_account_index = previous_config
            .active_account_index
            .saturating_sub(removed_before_active);
    }
    if previous_config.active_account_index >= previous_config.accounts.len() {
        previous_config.active_account_index = 0;
    }
}

pub(in crate::config) fn load_config_from_path(
    path: &Path,
) -> Result<Option<KeroseneConfig>, String> {
    match read_config_from_path(path) {
        Ok(Some(config)) => Ok(Some(config)),
        Ok(None) => recover_missing_primary_from_interrupted_save(path),
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

fn recover_missing_primary_from_interrupted_save(
    path: &Path,
) -> Result<Option<KeroseneConfig>, String> {
    let candidates = interrupted_save_rollback_candidates(path)?;
    if candidates.is_empty() {
        return Ok(None);
    }

    let mut load_errors = Vec::new();
    for candidate in &candidates {
        match read_config_from_path(candidate) {
            Ok(Some(config)) => {
                push_config_warning(format!(
                    "Loaded interrupted-save recovery config because primary config was missing: {}",
                    user_config_path(candidate)
                ));
                return Ok(Some(config));
            }
            Ok(None) => {}
            Err(error) => load_errors.push(error),
        }
    }

    let backup_path = backup_config_path(path);
    match read_config_from_path(&backup_path) {
        Ok(Some(config)) => {
            push_config_warning(
                "Loaded backup config because primary config was missing after an interrupted save"
                    .to_string(),
            );
            Ok(Some(config))
        }
        Ok(None) if load_errors.is_empty() => Ok(None),
        Ok(None) => Err(format!(
            "primary config missing after interrupted save; recovery sidecars failed: {}",
            load_errors.join("; ")
        )),
        Err(backup_error) if load_errors.is_empty() => Err(format!(
            "primary config missing after interrupted save; backup failed: {backup_error}"
        )),
        Err(backup_error) => Err(format!(
            "primary config missing after interrupted save; recovery sidecars failed: {}; backup failed: {backup_error}",
            load_errors.join("; ")
        )),
    }
}

fn interrupted_save_rollback_candidates(path: &Path) -> Result<Vec<PathBuf>, String> {
    let Some(parent) = path.parent() else {
        return Ok(Vec::new());
    };
    let Some(prefix) = config_sidecar_prefix(path, "replace-old") else {
        return Ok(Vec::new());
    };
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "scan config recovery sidecars in {} failed: {error}",
                user_config_dir()
            ));
        }
    };

    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "read config recovery sidecar entry in {} failed: {error}",
                user_config_dir()
            )
        })?;
        let file_name = entry.file_name();
        let is_candidate = file_name
            .to_str()
            .is_some_and(|name| name.starts_with(&prefix));
        if !is_candidate {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        candidates.push((modified, entry.path()));
    }

    candidates.sort_by(|(left_time, left_path), (right_time, right_path)| {
        right_time
            .cmp(left_time)
            .then_with(|| right_path.cmp(left_path))
    });
    Ok(candidates.into_iter().map(|(_, path)| path).collect())
}

pub(in crate::config) fn save_config_to_path(
    path: &Path,
    config: &KeroseneConfig,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create config directory {} failed: {e}", user_config_dir()))?;
    }
    let json = Zeroizing::new(
        serde_json::to_string_pretty(config)
            .map_err(|e| format!("serialize config failed: {e}"))?,
    );
    let temp_path = temp_config_path(path);
    write_with_restricted_permissions(&temp_path, json.as_bytes())?;

    let previous_config = read_config_from_path(path);
    let remove_stale_backup_after_save = matches!(previous_config, Ok(None));

    if let Ok(Some(previous_config)) = previous_config {
        let backup_path = backup_config_path(path);
        let backup_config = backup_config_for_save(previous_config, config);
        let backup_json = match serde_json::to_string_pretty(&backup_config) {
            Ok(json) => Zeroizing::new(json),
            Err(e) => {
                cleanup_file_best_effort(&temp_path);
                return Err(format!("sanitize backup config failed: {e}"));
            }
        };
        if let Err(e) = replace_with_restricted_permissions(&backup_path, backup_json.as_bytes()) {
            cleanup_file_best_effort(&temp_path);
            return Err(e);
        }
    }

    if let Err(e) = replace_temp_file(&temp_path, path, "replace config") {
        cleanup_file_best_effort(&temp_path);
        return if e.installed_replacement {
            Err(installed_snapshot_error(e.to_string()))
        } else {
            Err(e.to_string())
        };
    }

    if remove_stale_backup_after_save {
        remove_missing_primary_save_artifacts(path).map_err(installed_snapshot_error)?;
    }

    // The rename above carries the temp file's 0o600 onto `path`, so this
    // is belt-and-braces in case the prior on-disk config had looser
    // permissions for some reason.
    #[cfg(unix)]
    set_restricted_permissions(path).map_err(installed_snapshot_error)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;

    #[derive(Clone, Copy)]
    enum SecondInstallResult {
        Succeeds,
        Fails,
    }

    struct TestReplaceFileOps<'a> {
        temp: &'a Path,
        path: &'a Path,
        temp_install_attempts: &'a Cell<u32>,
        staged_rollback: &'a RefCell<Option<PathBuf>>,
        events: &'a RefCell<Vec<String>>,
        second_install_result: SecondInstallResult,
        replaceable_existing: bool,
        cleanup_fails: bool,
        sync_fails: bool,
    }

    impl ReplaceFileOps for TestReplaceFileOps<'_> {
        fn rename(&mut self, from: &Path, to: &Path) -> Result<(), String> {
            if from == self.temp && to == self.path {
                let next = self.temp_install_attempts.get() + 1;
                self.temp_install_attempts.set(next);
                self.events.borrow_mut().push(format!("install-{next}"));
                return if next == 1 {
                    Err("target exists".to_string())
                } else {
                    match self.second_install_result {
                        SecondInstallResult::Succeeds => Ok(()),
                        SecondInstallResult::Fails => Err("second rename failed".to_string()),
                    }
                };
            }

            if from == self.path {
                self.staged_rollback.borrow_mut().replace(to.to_path_buf());
                self.events.borrow_mut().push("stage-old".to_string());
                return Ok(());
            }

            if self
                .staged_rollback
                .borrow()
                .as_ref()
                .is_some_and(|staged| from == staged.as_path())
                && to == self.path
            {
                self.events.borrow_mut().push("restore-old".to_string());
                return Ok(());
            }

            Err(format!(
                "unexpected rename {} -> {}",
                from.display(),
                to.display()
            ))
        }

        fn exists(&mut self, candidate: &Path) -> bool {
            candidate == self.path
        }

        fn replaceable_existing(&mut self, _path: &Path) -> bool {
            self.replaceable_existing
        }

        fn cleanup_rollback(&mut self, rollback: &Path) -> Result<(), String> {
            assert!(
                self.staged_rollback
                    .borrow()
                    .as_ref()
                    .is_some_and(|staged| rollback == staged.as_path())
            );
            self.events.borrow_mut().push("cleanup-old".to_string());
            if self.cleanup_fails {
                Err("cleanup denied".to_string())
            } else {
                Ok(())
            }
        }

        fn sync_parent(&mut self, _path: &Path) -> Result<(), String> {
            self.events.borrow_mut().push("sync-parent".to_string());
            if self.sync_fails {
                Err("sync denied".to_string())
            } else {
                Ok(())
            }
        }
    }

    struct DirectInstallSyncFailureOps<'a> {
        temp: &'a Path,
        path: &'a Path,
        events: &'a RefCell<Vec<String>>,
    }

    impl ReplaceFileOps for DirectInstallSyncFailureOps<'_> {
        fn rename(&mut self, from: &Path, to: &Path) -> Result<(), String> {
            assert_eq!(from, self.temp);
            assert_eq!(to, self.path);
            self.events.borrow_mut().push("install".to_string());
            Ok(())
        }

        fn exists(&mut self, _path: &Path) -> bool {
            false
        }

        fn replaceable_existing(&mut self, _path: &Path) -> bool {
            false
        }

        fn cleanup_rollback(&mut self, _path: &Path) -> Result<(), String> {
            panic!("direct install should not stage rollback")
        }

        fn sync_parent(&mut self, _path: &Path) -> Result<(), String> {
            self.events.borrow_mut().push("sync-parent".to_string());
            Err("sync denied".to_string())
        }
    }

    #[test]
    fn primary_and_backup_sidecars_keep_distinct_file_names() {
        let path = PathBuf::from("/tmp/kerosene/config.json");
        let backup_path = backup_config_path(&path);

        let primary_temp = temp_config_path(&path);
        let backup_temp = temp_config_path(&backup_path);
        let primary_rollback = replacement_rollback_path(&path);
        let backup_rollback = replacement_rollback_path(&backup_path);

        assert_ne!(primary_temp, backup_temp);
        assert_ne!(primary_rollback, backup_rollback);
        assert!(
            primary_temp
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("config.json.tmp-"))
        );
        assert!(
            backup_temp
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("config.json.bak.tmp-"))
        );
        assert!(
            backup_rollback
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("config.json.bak.replace-old-"))
        );
    }

    #[test]
    fn replace_temp_file_restores_original_when_fallback_install_fails() {
        let temp = PathBuf::from("/tmp/kerosene-config.json.tmp");
        let path = PathBuf::from("/tmp/kerosene-config.json");
        let temp_install_attempts = Cell::new(0);
        let staged_rollback = RefCell::new(None::<PathBuf>);
        let events = RefCell::new(Vec::new());

        let result = replace_temp_file_with(
            &temp,
            &path,
            "replace config",
            TestReplaceFileOps {
                temp: &temp,
                path: &path,
                temp_install_attempts: &temp_install_attempts,
                staged_rollback: &staged_rollback,
                events: &events,
                second_install_result: SecondInstallResult::Fails,
                replaceable_existing: true,
                cleanup_fails: false,
                sync_fails: false,
            },
        );

        let error = result
            .expect_err("second install failure should be reported")
            .to_string();
        assert!(error.contains("second rename failed"));
        assert!(error.contains("original file was restored"));
        assert_eq!(
            events.borrow().as_slice(),
            ["install-1", "stage-old", "install-2", "restore-old"]
        );
    }

    #[test]
    fn replace_temp_file_marks_direct_sync_failure_after_install() {
        let temp = PathBuf::from("/tmp/kerosene-config.json.tmp");
        let path = PathBuf::from("/tmp/kerosene-config.json");
        let events = RefCell::new(Vec::new());

        let result = replace_temp_file_with(
            &temp,
            &path,
            "replace config",
            DirectInstallSyncFailureOps {
                temp: &temp,
                path: &path,
                events: &events,
            },
        );

        let error = result.expect_err("direct sync failure should be reported");
        assert!(error.installed_replacement);
        assert!(error.to_string().contains("sync after install failed"));
        assert_eq!(events.borrow().as_slice(), ["install", "sync-parent"]);
    }

    #[test]
    fn replace_temp_file_cleans_rollback_after_fallback_install_succeeds() {
        let temp = PathBuf::from("/tmp/kerosene-config.json.tmp");
        let path = PathBuf::from("/tmp/kerosene-config.json");
        let temp_install_attempts = Cell::new(0);
        let staged_rollback = RefCell::new(None::<PathBuf>);
        let events = RefCell::new(Vec::new());

        let result = replace_temp_file_with(
            &temp,
            &path,
            "replace config",
            TestReplaceFileOps {
                temp: &temp,
                path: &path,
                temp_install_attempts: &temp_install_attempts,
                staged_rollback: &staged_rollback,
                events: &events,
                second_install_result: SecondInstallResult::Succeeds,
                replaceable_existing: true,
                cleanup_fails: false,
                sync_fails: false,
            },
        );

        assert!(result.is_ok());
        assert_eq!(
            events.borrow().as_slice(),
            [
                "install-1",
                "stage-old",
                "install-2",
                "sync-parent",
                "cleanup-old",
                "sync-parent"
            ]
        );
    }

    #[test]
    fn replace_temp_file_reports_rollback_cleanup_failure() {
        let temp = PathBuf::from("/tmp/kerosene-config.json.tmp");
        let path = PathBuf::from("/tmp/kerosene-config.json");
        let temp_install_attempts = Cell::new(0);
        let staged_rollback = RefCell::new(None::<PathBuf>);
        let events = RefCell::new(Vec::new());

        let result = replace_temp_file_with(
            &temp,
            &path,
            "replace config",
            TestReplaceFileOps {
                temp: &temp,
                path: &path,
                temp_install_attempts: &temp_install_attempts,
                staged_rollback: &staged_rollback,
                events: &events,
                second_install_result: SecondInstallResult::Succeeds,
                replaceable_existing: true,
                cleanup_fails: true,
                sync_fails: false,
            },
        );

        let error = result
            .expect_err("rollback cleanup failure should be reported")
            .to_string();
        assert!(error.contains("replaced but cleanup rollback"));
        assert!(error.contains("cleanup denied"));
        assert_eq!(
            events.borrow().as_slice(),
            [
                "install-1",
                "stage-old",
                "install-2",
                "sync-parent",
                "cleanup-old"
            ]
        );
    }

    #[test]
    fn replace_temp_file_cleans_rollback_after_sync_failure() {
        let temp = PathBuf::from("/tmp/kerosene-config.json.tmp");
        let path = PathBuf::from("/tmp/kerosene-config.json");
        let temp_install_attempts = Cell::new(0);
        let staged_rollback = RefCell::new(None::<PathBuf>);
        let events = RefCell::new(Vec::new());

        let result = replace_temp_file_with(
            &temp,
            &path,
            "replace config",
            TestReplaceFileOps {
                temp: &temp,
                path: &path,
                temp_install_attempts: &temp_install_attempts,
                staged_rollback: &staged_rollback,
                events: &events,
                second_install_result: SecondInstallResult::Succeeds,
                replaceable_existing: true,
                cleanup_fails: false,
                sync_fails: true,
            },
        );

        let error = result
            .expect_err("sync failure should be reported")
            .to_string();
        assert!(error.contains("sync after install failed"));
        assert!(error.contains("rollback was cleaned up"));
        assert_eq!(
            events.borrow().as_slice(),
            [
                "install-1",
                "stage-old",
                "install-2",
                "sync-parent",
                "cleanup-old"
            ]
        );
    }

    #[test]
    fn replace_temp_file_does_not_stage_non_replaceable_existing_path() {
        let temp = PathBuf::from("/tmp/kerosene-config.json.tmp");
        let path = PathBuf::from("/tmp/kerosene-config.json");
        let temp_install_attempts = Cell::new(0);
        let staged_rollback = RefCell::new(None::<PathBuf>);
        let events = RefCell::new(Vec::new());

        let result = replace_temp_file_with(
            &temp,
            &path,
            "replace config",
            TestReplaceFileOps {
                temp: &temp,
                path: &path,
                temp_install_attempts: &temp_install_attempts,
                staged_rollback: &staged_rollback,
                events: &events,
                second_install_result: SecondInstallResult::Fails,
                replaceable_existing: false,
                cleanup_fails: false,
                sync_fails: false,
            },
        );

        let error = result
            .expect_err("non-replaceable target should fail")
            .to_string();
        assert!(error.contains("target exists"));
        assert_eq!(events.borrow().as_slice(), ["install-1"]);
    }
}
