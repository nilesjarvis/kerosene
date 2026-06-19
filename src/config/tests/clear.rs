use super::super::clear::{
    ConfigFileCleanupSummary, clear_all_configs_with, clear_config_files_at,
    clear_config_path_family,
};
use super::super::schema::AccountProfile;
use super::unique_test_config_path;
use std::cell::Cell;
use std::path::{Path, PathBuf};

fn sidecar_path(path: &Path, marker: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("test path should have a utf-8 file name");
    path.with_file_name(format!("{file_name}.{marker}-test"))
}

#[test]
fn clear_config_path_family_removes_primary_backup_and_temps() {
    let path = unique_test_config_path("clear-family");
    let parent = path.parent().expect("test path should have parent");
    let backup_path = super::super::files::backup_config_path(&path);
    let temp_path = sidecar_path(&path, "tmp");
    let backup_temp_path = sidecar_path(&backup_path, "tmp");
    let rollback_path = sidecar_path(&path, "replace-old");
    let backup_rollback_path = sidecar_path(&backup_path, "replace-old");
    let unrelated_path = parent.join("other.json.tmp-test");

    std::fs::create_dir_all(parent).expect("test directory can be created");
    std::fs::write(&path, "{}").expect("primary write succeeds");
    std::fs::write(&backup_path, "{}").expect("backup write succeeds");
    std::fs::write(&temp_path, "{}").expect("temp write succeeds");
    std::fs::write(&backup_temp_path, "{}").expect("backup temp write succeeds");
    std::fs::write(&rollback_path, "{}").expect("rollback write succeeds");
    std::fs::write(&backup_rollback_path, "{}").expect("backup rollback write succeeds");
    std::fs::write(&unrelated_path, "{}").expect("unrelated write succeeds");

    let removed = clear_config_path_family(&path).expect("clear should succeed");

    assert_eq!(removed, 6);
    assert!(!path.exists());
    assert!(!backup_path.exists());
    assert!(!temp_path.exists());
    assert!(!backup_temp_path.exists());
    assert!(!rollback_path.exists());
    assert!(!backup_rollback_path.exists());
    assert!(unrelated_path.exists());

    let _ = std::fs::remove_dir_all(parent);
}

#[test]
fn clear_config_files_removes_credential_and_account_cache_files() {
    let path = unique_test_config_path("clear-files-telegram-session");
    let parent = path.parent().expect("test path should have parent");
    let backup_path = super::super::files::backup_config_path(&path);
    let temp_path = sidecar_path(&path, "tmp");
    let backup_temp_path = sidecar_path(&backup_path, "tmp");
    let rollback_path = sidecar_path(&path, "replace-old");
    let backup_rollback_path = sidecar_path(&backup_path, "replace-old");
    let session_path = parent.join("telegram_fast.session");
    let session_shm_path = session_path.with_extension("session-shm");
    let session_wal_path = session_path.with_extension("session-wal");
    let session_journal_path = session_path.with_extension("session-journal");
    let journal_cache_path =
        parent.join("journal_cache_0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json");
    let journal_cache_temp_path =
        parent.join("journal_cache_0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json.tmp.1");
    let journal_cache_other_path =
        parent.join("journal_cache_0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.json");
    let unrelated_journal_path = parent.join("journal_cache_notes.txt");
    let unrelated_path = parent.join("telegram_fast.session.keep");

    std::fs::create_dir_all(parent).expect("test directory can be created");
    for candidate in [
        &path,
        &backup_path,
        &temp_path,
        &backup_temp_path,
        &rollback_path,
        &backup_rollback_path,
        &session_path,
        &session_shm_path,
        &session_wal_path,
        &session_journal_path,
        &journal_cache_path,
        &journal_cache_temp_path,
        &journal_cache_other_path,
        &unrelated_journal_path,
        &unrelated_path,
    ] {
        std::fs::write(candidate, "{}").expect("test file write succeeds");
    }

    let summary = clear_config_files_at(&path);

    assert_eq!(summary.files_removed, 13);
    assert!(!summary.file_cleanup_failed);
    assert!(summary.warnings.is_empty());
    for candidate in [
        &path,
        &backup_path,
        &temp_path,
        &backup_temp_path,
        &rollback_path,
        &backup_rollback_path,
        &session_path,
        &session_shm_path,
        &session_wal_path,
        &session_journal_path,
        &journal_cache_path,
        &journal_cache_temp_path,
        &journal_cache_other_path,
    ] {
        assert!(
            !candidate.exists(),
            "{} should be removed",
            candidate.display()
        );
    }
    assert!(unrelated_journal_path.exists());
    assert!(unrelated_path.exists());

    let _ = std::fs::remove_dir_all(parent);
}

#[test]
fn clear_config_files_marks_telegram_session_cleanup_failure_as_blocking() {
    let path = unique_test_config_path("clear-files-telegram-session-failure");
    let parent = path.parent().expect("test path should have parent");
    let session_path = parent.join("telegram_fast.session");

    std::fs::create_dir_all(parent).expect("test directory can be created");
    std::fs::write(&path, "{}").expect("config write succeeds");
    std::fs::create_dir_all(&session_path).expect("session dir can be created");

    let summary = clear_config_files_at(&path);

    assert_eq!(summary.files_removed, 1);
    assert!(summary.file_cleanup_failed);
    let warning = summary
        .warnings
        .iter()
        .find(|warning| warning.contains("Telegram session cleanup failed"))
        .expect("Telegram cleanup warning should be reported");
    assert!(warning.contains("<config-dir>/telegram_fast.session"));
    assert!(!warning.contains(&parent.display().to_string()));
    assert!(session_path.exists());

    let _ = std::fs::remove_dir_all(parent);
}

#[test]
fn clear_config_files_marks_journal_cache_cleanup_failure_as_blocking() {
    let path = unique_test_config_path("clear-files-journal-cache-failure");
    let parent = path.parent().expect("test path should have parent");
    let journal_cache_path =
        parent.join("journal_cache_0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json");

    std::fs::create_dir_all(parent).expect("test directory can be created");
    std::fs::write(&path, "{}").expect("config write succeeds");
    std::fs::create_dir_all(&journal_cache_path).expect("journal cache dir can be created");

    let summary = clear_config_files_at(&path);

    assert_eq!(summary.files_removed, 1);
    assert!(summary.file_cleanup_failed);
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.contains("journal cache cleanup failed"))
    );
    assert!(journal_cache_path.exists());

    let _ = std::fs::remove_dir_all(parent);
}

#[test]
fn clear_config_files_removes_imported_font_and_sound_dirs() {
    let path = unique_test_config_path("clear-files-imported-assets");
    let parent = path.parent().expect("test path should have parent");
    let font_dir = parent.join("fonts");
    let nested_font_dir = font_dir.join("nested");
    let sound_dir = parent.join("sounds");
    let unrelated_dir = parent.join("themes");
    let unrelated_file = unrelated_dir.join("keep.json");

    std::fs::create_dir_all(&nested_font_dir).expect("font directory can be created");
    std::fs::create_dir_all(&sound_dir).expect("sound directory can be created");
    std::fs::create_dir_all(&unrelated_dir).expect("unrelated directory can be created");
    std::fs::write(font_dir.join("custom.ttf"), "font").expect("font write succeeds");
    std::fs::write(nested_font_dir.join("nested.otf"), "font").expect("nested font write succeeds");
    std::fs::write(sound_dir.join("alert.wav"), "sound").expect("sound write succeeds");
    std::fs::write(&unrelated_file, "{}").expect("unrelated file write succeeds");

    let summary = clear_config_files_at(&path);

    assert_eq!(summary.files_removed, 2);
    assert!(!summary.file_cleanup_failed);
    assert!(summary.warnings.is_empty());
    assert!(!font_dir.exists());
    assert!(!sound_dir.exists());
    assert!(unrelated_dir.exists());
    assert!(unrelated_file.exists());

    let _ = std::fs::remove_dir_all(parent);
}

#[test]
fn clear_config_files_preserves_ancillary_files_when_primary_config_cleanup_fails() {
    let path = unique_test_config_path("clear-files-primary-failure");
    let parent = path.parent().expect("test path should have parent");
    let session_path = parent.join("telegram_fast.session");
    let journal_cache_path =
        parent.join("journal_cache_0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json");
    let font_dir = parent.join("fonts");

    std::fs::create_dir_all(&path).expect("config path directory can be created");
    std::fs::write(&session_path, "{}").expect("session write succeeds");
    std::fs::write(&journal_cache_path, "{}").expect("journal cache write succeeds");
    std::fs::create_dir_all(&font_dir).expect("font dir can be created");
    std::fs::write(font_dir.join("custom.ttf"), "font").expect("font write succeeds");

    let summary = clear_config_files_at(&path);

    assert_eq!(summary.files_removed, 0);
    assert!(summary.file_cleanup_failed);
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.contains("config file cleanup failed"))
    );
    assert!(session_path.exists());
    assert!(journal_cache_path.exists());
    assert!(font_dir.exists());
    assert!(path.exists());

    let _ = std::fs::remove_dir_all(parent);
}

#[test]
fn clear_all_configs_runs_keychain_before_primary_config_cleanup() {
    let profiles = vec![account("account-a"), account("account-b")];
    let keychain_called = Cell::new(false);
    let side_cleanup_called = Cell::new(false);

    let summary = clear_all_configs_with(
        &profiles,
        |_| {
            keychain_called.set(true);
            Ok(())
        },
        || cleanup_failure("remove config failed"),
        || {
            side_cleanup_called.set(true);
            cleanup_success(5)
        },
    );

    assert_eq!(summary.files_removed, 0);
    assert!(summary.file_cleanup_failed);
    assert_eq!(summary.keychain_entries_cleared, 8);
    assert!(keychain_called.get());
    assert!(!side_cleanup_called.get());
    assert_eq!(
        summary.warnings,
        vec!["config file cleanup failed: remove config failed".to_string()]
    );
}

#[test]
fn clear_all_configs_runs_keychain_before_config_and_side_cleanup() {
    let profiles = vec![account("account-a")];
    let step = Cell::new(0);

    let summary = clear_all_configs_with(
        &profiles,
        |_| {
            assert_eq!(step.get(), 0);
            step.set(1);
            Ok(())
        },
        || {
            assert_eq!(step.get(), 1);
            step.set(2);
            cleanup_success(2)
        },
        || {
            assert_eq!(step.get(), 2);
            step.set(3);
            cleanup_success(3)
        },
    );

    assert_eq!(step.get(), 3);
    assert_eq!(summary.files_removed, 5);
    assert!(!summary.file_cleanup_failed);
    assert_eq!(summary.keychain_entries_cleared, 6);
    assert!(summary.warnings.is_empty());
}

#[test]
fn clear_all_configs_reports_keychain_failure_before_config_cleanup() {
    let profiles = vec![account("account-a"), account("")];
    let primary_cleanup_called = Cell::new(false);
    let side_cleanup_called = Cell::new(false);

    let summary = clear_all_configs_with(
        &profiles,
        |_| Err("keychain unavailable: api_key=super-secret".to_string()),
        || {
            primary_cleanup_called.set(true);
            cleanup_success(2)
        },
        || {
            side_cleanup_called.set(true);
            cleanup_success(5)
        },
    );

    assert_eq!(summary.files_removed, 0);
    assert!(summary.file_cleanup_failed);
    assert_eq!(summary.keychain_entries_cleared, 0);
    assert!(!primary_cleanup_called.get());
    assert!(!side_cleanup_called.get());
    assert_eq!(
        summary.warnings,
        vec!["keychain cleanup failed: keychain unavailable: api_key=<redacted>".to_string()]
    );
    assert!(!summary.warnings.join("; ").contains("super-secret"));
}

#[test]
fn clear_all_configs_reports_sensitive_side_cleanup_failure_after_keychain_cleanup() {
    let profiles = vec![account("account-a")];
    let keychain_called = Cell::new(false);

    let summary = clear_all_configs_with(
        &profiles,
        |_| {
            keychain_called.set(true);
            Ok(())
        },
        || cleanup_success(1),
        || ConfigFileCleanupSummary {
            files_removed: 0,
            file_cleanup_failed: true,
            warnings: vec!["Telegram session cleanup failed: permission denied".to_string()],
        },
    );

    assert_eq!(summary.files_removed, 1);
    assert!(summary.file_cleanup_failed);
    assert_eq!(summary.keychain_entries_cleared, 6);
    assert!(keychain_called.get());
    assert_eq!(
        summary.warnings,
        vec!["Telegram session cleanup failed: permission denied".to_string()]
    );
}

#[test]
fn clear_all_configs_skips_side_cleanup_when_keychain_cleanup_fails() {
    let profiles = vec![account("account-a")];
    let primary_cleanup_called = Cell::new(false);
    let side_cleanup_called = Cell::new(false);

    let summary = clear_all_configs_with(
        &profiles,
        |_| Err("keychain locked: auth_token=super-secret".to_string()),
        || {
            primary_cleanup_called.set(true);
            cleanup_success(3)
        },
        || {
            side_cleanup_called.set(true);
            cleanup_success(3)
        },
    );

    assert_eq!(summary.files_removed, 0);
    assert!(summary.file_cleanup_failed);
    assert_eq!(summary.keychain_entries_cleared, 0);
    assert!(!primary_cleanup_called.get());
    assert!(!side_cleanup_called.get());
    assert_eq!(
        summary.warnings,
        vec!["keychain cleanup failed: keychain locked: auth_token=<redacted>".to_string()]
    );
    assert!(!summary.warnings.join("; ").contains("super-secret"));
}

#[test]
fn clear_all_configs_runs_keychain_when_only_ancillary_cleanup_warns() {
    let profiles = vec![account("account-a")];
    let keychain_called = Cell::new(false);

    let summary = clear_all_configs_with(
        &profiles,
        |_| {
            keychain_called.set(true);
            Ok(())
        },
        || cleanup_success(3),
        || ConfigFileCleanupSummary {
            files_removed: 1,
            file_cleanup_failed: false,
            warnings: vec!["asset cleanup failed: remove fonts failed".to_string()],
        },
    );

    assert_eq!(summary.files_removed, 4);
    assert!(!summary.file_cleanup_failed);
    assert_eq!(summary.keychain_entries_cleared, 6);
    assert!(keychain_called.get());
    assert_eq!(
        summary.warnings,
        vec!["asset cleanup failed: remove fonts failed".to_string()]
    );
}

fn cleanup_success(files_removed: usize) -> ConfigFileCleanupSummary {
    ConfigFileCleanupSummary {
        files_removed,
        file_cleanup_failed: false,
        warnings: Vec::new(),
    }
}

fn cleanup_failure(error: &str) -> ConfigFileCleanupSummary {
    ConfigFileCleanupSummary {
        files_removed: 0,
        file_cleanup_failed: true,
        warnings: vec![format!("config file cleanup failed: {error}")],
    }
}

fn account(secret_id: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: "Test".to_string(),
        wallet_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    }
}
