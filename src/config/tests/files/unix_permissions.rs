use super::*;

#[test]
fn write_with_restricted_permissions_creates_file_with_owner_only_mode() {
    let path = test_path("restricted-write");
    create_parent_dir(&path);

    write_restricted(&path, b"secret-payload");

    assert_owner_only_mode(&path, "restricted write mode");

    cleanup_path(&path);
}

#[test]
fn write_with_restricted_permissions_refuses_existing_world_readable_file() {
    let path = test_path("restricted-existing");
    create_parent_dir(&path);

    write_file(&path, b"old-leaky-contents");
    set_file_mode(&path, 0o644);

    let err = match write_with_restricted_permissions(&path, b"new-secret-payload") {
        Ok(()) => panic!("restricted writer should not rewrite existing files"),
        Err(err) => err,
    };
    assert!(
        err.contains("create") || err.contains("exists"),
        "unexpected error: {err}"
    );

    assert_eq!(file_mode(&path) & 0o777, 0o644);
    assert_eq!(read_file(&path), b"old-leaky-contents");

    cleanup_path(&path);
}

#[test]
fn save_config_replaces_existing_loose_backup_without_reusing_its_mode() {
    let path = test_path("loose-backup-replacement");
    let backup_path = backup_config_path(&path);
    let mut config = address_book_config(
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "Initial Primary",
        None,
        &[],
    );

    save_config_fixture(&path, &config);
    write_file(&backup_path, b"old world-readable backup");
    set_file_mode(&backup_path, 0o644);

    config.address_book[0].label = "Updated Primary".to_string();
    save_config_fixture(&path, &config);

    assert_owner_only_mode(&backup_path, "backup config mode");

    let backup = load_existing_config(&backup_path);
    assert_eq!(backup.address_book[0].label, "Initial Primary");

    let primary = load_existing_config(&path);
    assert_eq!(primary.address_book[0].label, "Updated Primary");

    cleanup_path(&path);
}

#[test]
fn save_config_writes_primary_and_backup_with_owner_only_mode() {
    let path = test_path("perms-end-to-end");

    let config = address_book_config(
        "0xcccccccccccccccccccccccccccccccccccccccc",
        "Perms",
        None,
        &[],
    );

    // First save: no prior config, no backup created.
    save_config_fixture(&path, &config);
    assert_owner_only_mode(&path, "primary config mode");

    // Second save: backup is now produced from the prior primary.
    save_config_fixture(&path, &config);
    assert_owner_only_mode(&backup_config_path(&path), "backup config mode");

    cleanup_path(&path);
}
