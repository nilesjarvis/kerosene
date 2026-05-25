use super::*;

#[test]
fn config_save_round_trips_wallet_labels_and_keeps_backup() {
    let path = test_path("round-trip");
    let mut config = address_book_config(
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "Alpha",
        Some("#FF7A1A"),
        &["desk"],
    );

    save_config_fixture(&path, &config);
    let loaded = load_existing_config(&path);
    assert_eq!(loaded.address_book, config.address_book);

    config.address_book[0].label = "Beta".to_string();
    save_config_fixture(&path, &config);

    let loaded = load_existing_config(&path);
    assert_eq!(loaded.address_book[0].label, "Beta");

    let backup = load_existing_config(&backup_config_path(&path));
    assert_eq!(backup.address_book[0].label, "Alpha");

    cleanup_path(&path);
}

#[test]
fn config_load_falls_back_to_backup_when_primary_is_corrupt() {
    let _ = take_config_warnings();
    let path = test_path("backup");
    let config = address_book_config(
        "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "Backup",
        None,
        &[],
    );

    save_config_fixture(&path, &config);
    let backup_path = backup_config_path(&path);
    write_file(&backup_path, config_json(&config));
    write_file(&path, "{not json");

    let loaded = load_existing_config(&path);
    assert_eq!(loaded.address_book[0].label, "Backup");
    assert!(
        take_config_warnings()
            .iter()
            .any(|warning| warning.contains("Loaded backup config"))
    );

    cleanup_path(&path);
}

#[test]
fn config_save_does_not_replace_valid_backup_with_corrupt_primary() {
    let path = test_path("preserve-backup");
    let backup_path = backup_config_path(&path);
    let backup_config = address_book_config(
        "0xcccccccccccccccccccccccccccccccccccccccc",
        "Good Backup",
        None,
        &[],
    );

    create_parent_dir(&path);
    write_file(&path, "{not json");
    write_file(&backup_path, config_json(&backup_config));

    let new_config = address_book_config(
        "0xdddddddddddddddddddddddddddddddddddddddd",
        "New Primary",
        None,
        &[],
    );
    save_config_fixture(&path, &new_config);

    let loaded_primary = load_existing_config(&path);
    let loaded_backup = load_existing_config(&backup_path);

    assert_eq!(loaded_primary.address_book[0].label, "New Primary");
    assert_eq!(loaded_backup.address_book[0].label, "Good Backup");

    cleanup_path(&path);
}
