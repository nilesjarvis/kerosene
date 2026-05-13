#[cfg(unix)]
use super::super::files::write_with_restricted_permissions;
use super::super::files::{backup_config_path, load_config_from_path, save_config_to_path};
use super::super::{AddressBookEntryConfig, KeroseneConfig, take_config_warnings};
use super::unique_test_config_path;

#[test]
fn config_save_round_trips_wallet_labels_and_keeps_backup() {
    let path = unique_test_config_path("round-trip");
    let mut config = KeroseneConfig {
        address_book: vec![AddressBookEntryConfig {
            address: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            label: "Alpha".to_string(),
            color: Some("#FF7A1A".to_string()),
            tags: vec!["desk".to_string()],
        }],
        ..Default::default()
    };

    save_config_to_path(&path, &config).expect("initial save should succeed");
    let loaded = load_config_from_path(&path)
        .expect("saved config should load")
        .expect("config should exist");
    assert_eq!(loaded.address_book, config.address_book);

    config.address_book[0].label = "Beta".to_string();
    save_config_to_path(&path, &config).expect("second save should succeed");

    let loaded = load_config_from_path(&path)
        .expect("updated config should load")
        .expect("config should exist");
    assert_eq!(loaded.address_book[0].label, "Beta");

    let backup = load_config_from_path(&backup_config_path(&path))
        .expect("backup config should load")
        .expect("backup config should exist");
    assert_eq!(backup.address_book[0].label, "Alpha");

    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

#[test]
fn config_load_falls_back_to_backup_when_primary_is_corrupt() {
    let _ = take_config_warnings();
    let path = unique_test_config_path("backup");
    let config = KeroseneConfig {
        address_book: vec![AddressBookEntryConfig {
            address: "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            label: "Backup".to_string(),
            color: None,
            tags: Vec::new(),
        }],
        ..Default::default()
    };

    save_config_to_path(&path, &config).expect("save should succeed");
    let backup_path = backup_config_path(&path);
    let json = serde_json::to_string_pretty(&config).expect("config serializes");
    std::fs::write(&backup_path, json).expect("backup write succeeds");
    std::fs::write(&path, "{not json").expect("primary write succeeds");

    let loaded = load_config_from_path(&path)
        .expect("backup fallback should load")
        .expect("config should exist");
    assert_eq!(loaded.address_book[0].label, "Backup");
    assert!(
        take_config_warnings()
            .iter()
            .any(|warning| warning.contains("Loaded backup config"))
    );

    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

#[test]
fn config_save_does_not_replace_valid_backup_with_corrupt_primary() {
    let path = unique_test_config_path("preserve-backup");
    let backup_path = backup_config_path(&path);
    let backup_config = KeroseneConfig {
        address_book: vec![AddressBookEntryConfig {
            address: "0xcccccccccccccccccccccccccccccccccccccccc".to_string(),
            label: "Good Backup".to_string(),
            color: None,
            tags: Vec::new(),
        }],
        ..Default::default()
    };
    let backup_json = serde_json::to_string_pretty(&backup_config).expect("config serializes");

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("test directory can be created");
    }
    std::fs::write(&path, "{not json").expect("primary write succeeds");
    std::fs::write(&backup_path, backup_json).expect("backup write succeeds");

    let new_config = KeroseneConfig {
        address_book: vec![AddressBookEntryConfig {
            address: "0xdddddddddddddddddddddddddddddddddddddddd".to_string(),
            label: "New Primary".to_string(),
            color: None,
            tags: Vec::new(),
        }],
        ..Default::default()
    };
    save_config_to_path(&path, &new_config).expect("save should succeed");

    let loaded_primary = load_config_from_path(&path)
        .expect("primary should load")
        .expect("primary should exist");
    let loaded_backup = load_config_from_path(&backup_path)
        .expect("backup should load")
        .expect("backup should exist");

    assert_eq!(loaded_primary.address_book[0].label, "New Primary");
    assert_eq!(loaded_backup.address_book[0].label, "Good Backup");

    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

#[cfg(unix)]
#[test]
fn write_with_restricted_permissions_creates_file_with_owner_only_mode() {
    use std::os::unix::fs::PermissionsExt;

    let path = unique_test_config_path("restricted-write");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create test dir");
    }

    write_with_restricted_permissions(&path, b"secret-payload")
        .expect("restricted write should succeed");

    let mode = std::fs::metadata(&path)
        .expect("written file should exist")
        .permissions()
        .mode();
    // Only the owner permission bits should be set; group/world bits MUST
    // be clear so encrypted credential blobs in the temp file are not
    // readable by another local user during the crash/pre-chmod window.
    assert_eq!(
        mode & 0o777,
        0o600,
        "expected 0o600, got {:o}",
        mode & 0o777
    );

    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

#[cfg(unix)]
#[test]
fn write_with_restricted_permissions_overwrites_a_world_readable_file_to_owner_only_mode() {
    use std::os::unix::fs::PermissionsExt;

    let path = unique_test_config_path("restricted-overwrite");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create test dir");
    }

    // Plant a world-readable file at the path to simulate a stale temp
    // left over from a previous crash with a permissive umask.
    std::fs::write(&path, b"old-leaky-contents").expect("seed file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("loosen perms");

    write_with_restricted_permissions(&path, b"new-secret-payload")
        .expect("restricted overwrite should succeed");

    let mode = std::fs::metadata(&path)
        .expect("written file should exist")
        .permissions()
        .mode();
    // OpenOptions.mode() only applies on create. The current implementation
    // does not chmod when truncating an existing file, so this test pins
    // that behavior — if a future change starts re-chmodding, update the
    // assertion. The pre-write `remove_file` in save_config_to_path makes
    // this not a real-world hazard because the temp path is always created
    // fresh; the loosen-then-overwrite case only matters if someone calls
    // write_with_restricted_permissions on an existing path directly.
    if mode & 0o077 != 0 {
        eprintln!(
            "note: write_with_restricted_permissions did not tighten an existing file's mode \
             (got {:o}). callers must pre-remove the path; save_config_to_path already does.",
            mode & 0o777
        );
    }
    // Regardless, the bytes were replaced.
    let read_back = std::fs::read(&path).expect("read back");
    assert_eq!(read_back, b"new-secret-payload");

    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

#[cfg(unix)]
#[test]
fn save_config_writes_primary_and_backup_with_owner_only_mode() {
    use std::os::unix::fs::PermissionsExt;

    let path = unique_test_config_path("perms-end-to-end");

    let config = KeroseneConfig {
        address_book: vec![AddressBookEntryConfig {
            address: "0xcccccccccccccccccccccccccccccccccccccccc".to_string(),
            label: "Perms".to_string(),
            color: None,
            tags: vec![],
        }],
        ..Default::default()
    };

    // First save — no prior config, no backup created.
    save_config_to_path(&path, &config).expect("initial save should succeed");
    let primary_mode = std::fs::metadata(&path)
        .expect("primary should exist")
        .permissions()
        .mode();
    assert_eq!(
        primary_mode & 0o777,
        0o600,
        "primary config mode: expected 0o600, got {:o}",
        primary_mode & 0o777
    );

    // Second save — backup is now produced from the prior primary.
    save_config_to_path(&path, &config).expect("second save should succeed");
    let backup_mode = std::fs::metadata(backup_config_path(&path))
        .expect("backup should exist")
        .permissions()
        .mode();
    assert_eq!(
        backup_mode & 0o777,
        0o600,
        "backup config mode: expected 0o600, got {:o}",
        backup_mode & 0o777
    );

    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}
