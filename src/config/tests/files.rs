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
