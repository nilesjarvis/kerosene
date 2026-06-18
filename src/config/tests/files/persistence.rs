use super::*;

use crate::config::{
    AccountProfile, CredentialStorageMode, ReadDataProvider, SecretPayload, decrypt_secrets,
    encrypt_secrets,
};

#[test]
fn config_save_round_trips_wallet_labels_and_keeps_backup() {
    let _warning_guard = config_warning_guard();
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
    let _warning_guard = config_warning_guard();
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
fn config_load_ignores_backup_when_primary_is_missing() {
    let _warning_guard = config_warning_guard();
    let path = test_path("missing-primary-backup");
    let backup_path = backup_config_path(&path);
    let config = address_book_config(
        "0xbabababababababababababababababababababa",
        "Missing Primary Backup",
        None,
        &[],
    );

    create_parent_dir(&path);
    write_file(&backup_path, config_json(&config));

    let loaded = load_config_from_path(&path).expect("missing primary should not fail");
    assert!(loaded.is_none());
    assert!(take_config_warnings().is_empty());

    cleanup_path(&path);
}

#[test]
fn config_load_defaults_unknown_read_data_provider() {
    let _warning_guard = config_warning_guard();
    let path = test_path("unknown-read-provider");
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    value
        .as_object_mut()
        .expect("default config should serialize as object")
        .insert(
            "read_data_provider".to_string(),
            serde_json::json!("FutureProvider"),
        );

    create_parent_dir(&path);
    write_file(
        &path,
        serde_json::to_string_pretty(&value).expect("test config should encode"),
    );

    let loaded = load_existing_config(&path);

    assert_eq!(loaded.read_data_provider, ReadDataProvider::Hyperliquid);
    assert!(
        take_config_warnings()
            .iter()
            .any(|warning| warning.contains("Unknown read data provider \"FutureProvider\""))
    );

    cleanup_path(&path);
}

#[test]
fn config_load_recovers_missing_primary_from_interrupted_replace_sidecar() {
    let _warning_guard = config_warning_guard();
    let path = test_path("missing-primary-rollback");
    let rollback_path = interrupted_replace_sidecar_path(&path, "newer");
    let backup_path = backup_config_path(&path);
    let rollback_config = address_book_config(
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "Rollback Primary",
        None,
        &[],
    );
    let backup_config = address_book_config(
        "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "Backup",
        None,
        &[],
    );

    create_parent_dir(&path);
    write_file(&rollback_path, config_json(&rollback_config));
    write_file(&backup_path, config_json(&backup_config));

    let loaded = load_existing_config(&path);

    assert_eq!(loaded.address_book[0].label, "Rollback Primary");
    assert!(
        take_config_warnings()
            .iter()
            .any(|warning| warning.contains("interrupted-save recovery config"))
    );

    cleanup_path(&path);
}

#[test]
fn config_load_uses_backup_for_missing_primary_when_interrupted_sidecar_is_invalid() {
    let _warning_guard = config_warning_guard();
    let path = test_path("missing-primary-invalid-rollback");
    let rollback_path = interrupted_replace_sidecar_path(&path, "invalid");
    let backup_path = backup_config_path(&path);
    let backup_config = address_book_config(
        "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "Backup Recovery",
        None,
        &[],
    );

    create_parent_dir(&path);
    write_file(&rollback_path, "{not json");
    write_file(&backup_path, config_json(&backup_config));

    let loaded = load_existing_config(&path);

    assert_eq!(loaded.address_book[0].label, "Backup Recovery");
    assert!(
        take_config_warnings()
            .iter()
            .any(|warning| warning.contains("primary config was missing after an interrupted save"))
    );

    cleanup_path(&path);
}

#[test]
fn config_save_after_missing_primary_removes_stale_recovery_artifacts() {
    let _warning_guard = config_warning_guard();
    let path = test_path("missing-primary-save-removes-stale-artifacts");
    let backup_path = backup_config_path(&path);
    let primary_rollback_path = interrupted_replace_sidecar_path(&path, "primary");
    let backup_rollback_path = interrupted_replace_sidecar_path(&backup_path, "backup");
    let primary_temp_path = interrupted_temp_sidecar_path(&path, "primary");
    let backup_temp_path = interrupted_temp_sidecar_path(&backup_path, "backup");
    let stale_config = address_book_config(
        "0xbabababababababababababababababababababa",
        "Stale Recovery",
        None,
        &[],
    );
    let new_config = address_book_config(
        "0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
        "New Primary",
        None,
        &[],
    );

    create_parent_dir(&path);
    write_file(&backup_path, config_json(&stale_config));
    write_file(&primary_rollback_path, config_json(&stale_config));
    write_file(&backup_rollback_path, config_json(&stale_config));
    write_file(&primary_temp_path, "{stale primary temp");
    write_file(&backup_temp_path, "{stale backup temp");

    save_config_fixture(&path, &new_config);
    for artifact in [
        &backup_path,
        &primary_rollback_path,
        &backup_rollback_path,
        &primary_temp_path,
        &backup_temp_path,
    ] {
        assert!(
            !artifact.exists(),
            "stale recovery artifact should be removed: {}",
            artifact.display()
        );
    }

    std::fs::remove_file(&path).expect("remove saved primary to test stale resurrection");
    let loaded = load_config_from_path(&path).expect("missing primary should not fail");
    assert!(
        loaded.is_none(),
        "stale recovery artifacts should not resurrect"
    );
    assert!(take_config_warnings().is_empty());

    cleanup_path(&path);
}

fn interrupted_replace_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = path
        .file_name()
        .expect("test config path should have a file name")
        .to_os_string();
    file_name.push(format!(".replace-old-{suffix}"));
    path.parent()
        .expect("test config path should have a parent")
        .join(file_name)
}

fn interrupted_temp_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = path
        .file_name()
        .expect("test config path should have a file name")
        .to_os_string();
    file_name.push(format!(".tmp-{suffix}"));
    path.parent()
        .expect("test config path should have a parent")
        .join(file_name)
}

#[test]
fn config_save_does_not_replace_valid_backup_with_corrupt_primary() {
    let _warning_guard = config_warning_guard();
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

#[test]
fn config_save_sanitizes_legacy_plaintext_secrets_before_backup() {
    let _warning_guard = config_warning_guard();
    let path = test_path("backup-secret-scrub");
    create_parent_dir(&path);
    let mut legacy = serde_json::to_value(KeroseneConfig::default())
        .expect("default config should serialize to json");
    let object = legacy
        .as_object_mut()
        .expect("default config should serialize as object");
    object.insert(
        "accounts".to_string(),
        serde_json::json!([{
            "secret_id": "acct-a",
            "name": "Main",
            "wallet_address": "",
            "agent_key": "agent-secret",
            "hydromancer_api_key": "profile-hydro-secret"
        }]),
    );
    object.insert(
        "agent_key".to_string(),
        serde_json::json!("legacy-agent-secret"),
    );
    object.insert(
        "hydromancer_api_key".to_string(),
        serde_json::json!("hydro-secret"),
    );
    object.insert(
        "hyperdash_api_key".to_string(),
        serde_json::json!("hyper-secret"),
    );
    object.insert("x_bearer_token".to_string(), serde_json::json!("x-secret"));
    write_file(
        &path,
        serde_json::to_string_pretty(&legacy).expect("legacy config should encode"),
    );

    save_config_fixture(&path, &KeroseneConfig::default());

    let primary = std::fs::read_to_string(&path).expect("primary config should be readable");
    let backup = std::fs::read_to_string(backup_config_path(&path))
        .expect("backup config should be readable");
    for secret in [
        "agent-secret",
        "profile-hydro-secret",
        "legacy-agent-secret",
        "hydro-secret",
        "hyper-secret",
        "x-secret",
    ] {
        assert!(!primary.contains(secret), "primary leaked {secret}");
        assert!(!backup.contains(secret), "backup leaked {secret}");
    }

    cleanup_path(&path);
}

#[test]
fn config_save_updates_backup_encrypted_secret_blob_when_secrets_change() {
    let _warning_guard = config_warning_guard();
    let path = test_path("backup-encrypted-secret-refresh");
    let password = "correct horse";
    let mut first_config = encrypted_config_fixture(
        &[
            (
                "acct-a",
                "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "agent-a",
            ),
            (
                "acct-b",
                "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "agent-b",
            ),
        ],
        password,
    );

    save_config_fixture(&path, &first_config);

    first_config.address_book = address_book_config(
        "0xcccccccccccccccccccccccccccccccccccccccc",
        "Previous",
        None,
        &[],
    )
    .address_book;
    let next_config = encrypted_config_fixture(
        &[(
            "acct-a",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "agent-a",
        )],
        password,
    );

    save_config_fixture(&path, &first_config);
    save_config_fixture(&path, &next_config);

    let backup = load_existing_config(&backup_config_path(&path));
    assert_eq!(backup.address_book[0].label, "Previous");
    assert_eq!(
        backup
            .accounts
            .iter()
            .map(|profile| profile.secret_id.as_str())
            .collect::<Vec<_>>(),
        ["acct-a"]
    );

    let backup_secrets = backup
        .encrypted_secrets
        .as_ref()
        .expect("backup should keep current encrypted secret blob");
    let payload = decrypt_secrets(backup_secrets, password).expect("backup secrets should decrypt");

    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
    assert_eq!(payload.profile_agent_key("acct-b"), None);
    assert!(!config_json(&backup).contains("agent-b"));

    write_file(&path, "{not json");
    let loaded = load_existing_config(&path);
    assert_eq!(
        loaded
            .accounts
            .iter()
            .map(|profile| profile.secret_id.as_str())
            .collect::<Vec<_>>(),
        ["acct-a"]
    );

    cleanup_path(&path);
}

#[test]
fn config_save_updates_backup_secret_storage_fields_when_switching_to_encrypted_config() {
    let _warning_guard = config_warning_guard();
    let path = test_path("backup-storage-mode-switch");
    let password = "correct horse";
    let previous_config = address_book_config(
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "OS Keychain Primary",
        None,
        &[],
    );
    let mut next_config = encrypted_config_fixture(
        &[(
            "acct-a",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "agent-a",
        )],
        password,
    );
    next_config.pending_keychain_cleanup_all = true;

    assert_eq!(
        previous_config.credential_storage_mode,
        CredentialStorageMode::OsKeychain
    );
    save_config_fixture(&path, &previous_config);
    save_config_fixture(&path, &next_config);

    let backup = load_existing_config(&backup_config_path(&path));
    assert_eq!(backup.address_book[0].label, "OS Keychain Primary");
    assert_eq!(
        backup.credential_storage_mode,
        CredentialStorageMode::EncryptedConfig
    );
    assert!(backup.pending_keychain_cleanup_all);
    let backup_secrets = backup
        .encrypted_secrets
        .as_ref()
        .expect("backup should inherit encrypted secret blob after storage switch");
    let payload = decrypt_secrets(backup_secrets, password).expect("backup secrets should decrypt");

    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
    assert!(!config_json(&backup).contains("agent-a"));

    cleanup_path(&path);
}

#[test]
fn backup_config_for_pending_keychain_delete_does_not_resurrect_deleted_account() {
    let _warning_guard = config_warning_guard();
    let path = test_path("backup-pending-keychain-delete");
    let previous_config = KeroseneConfig {
        accounts: vec![
            AccountProfile {
                secret_id: "acct-a".to_string(),
                name: "Account A".to_string(),
                wallet_address: String::new(),
                agent_key: String::new().into(),
                hydromancer_api_key: String::new().into(),
            },
            AccountProfile {
                secret_id: "acct-b".to_string(),
                name: "Account B".to_string(),
                wallet_address: String::new(),
                agent_key: String::new().into(),
                hydromancer_api_key: String::new().into(),
            },
        ],
        active_account_index: 1,
        ..KeroseneConfig::default()
    };
    let next_config = KeroseneConfig {
        accounts: previous_config.accounts[..1].to_vec(),
        pending_keychain_profile_deletions: vec!["acct-b".to_string()],
        active_account_index: 0,
        ..KeroseneConfig::default()
    };

    save_config_fixture(&path, &previous_config);
    save_config_fixture(&path, &next_config);

    let backup = load_existing_config(&backup_config_path(&path));
    assert_eq!(
        backup
            .accounts
            .iter()
            .map(|profile| profile.secret_id.as_str())
            .collect::<Vec<_>>(),
        ["acct-a"]
    );
    assert_eq!(
        backup.pending_keychain_profile_deletions.as_slice(),
        ["acct-b"]
    );

    write_file(&path, "{not json");
    let loaded = load_existing_config(&path);
    assert_eq!(
        loaded
            .accounts
            .iter()
            .map(|profile| profile.secret_id.as_str())
            .collect::<Vec<_>>(),
        ["acct-a"]
    );
    assert_eq!(
        loaded.pending_keychain_profile_deletions.as_slice(),
        ["acct-b"]
    );

    cleanup_path(&path);
}

#[test]
fn backup_config_clears_pending_keychain_delete_after_cleanup_state_save() {
    let _warning_guard = config_warning_guard();
    let path = test_path("backup-pending-keychain-delete-cleared");
    let account_a = AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: String::new(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    };
    let previous_config = KeroseneConfig {
        accounts: vec![account_a.clone()],
        pending_keychain_profile_deletions: vec!["acct-b".to_string()],
        ..KeroseneConfig::default()
    };
    let next_config = KeroseneConfig {
        accounts: vec![account_a],
        ..KeroseneConfig::default()
    };

    save_config_fixture(&path, &previous_config);
    save_config_fixture(&path, &next_config);

    let backup = load_existing_config(&backup_config_path(&path));
    assert!(backup.pending_keychain_profile_deletions.is_empty());

    cleanup_path(&path);
}

#[test]
fn backup_config_clears_pending_full_keychain_cleanup_after_cleanup_state_save() {
    let _warning_guard = config_warning_guard();
    let path = test_path("backup-full-keychain-cleanup-cleared");
    let previous_config = KeroseneConfig {
        pending_keychain_cleanup_all: true,
        ..KeroseneConfig::default()
    };
    let next_config = KeroseneConfig::default();

    save_config_fixture(&path, &previous_config);
    save_config_fixture(&path, &next_config);

    let backup = load_existing_config(&backup_config_path(&path));
    assert!(!backup.pending_keychain_cleanup_all);

    cleanup_path(&path);
}

fn encrypted_config_fixture(profiles: &[(&str, &str, &str)], password: &str) -> KeroseneConfig {
    let accounts: Vec<_> = profiles
        .iter()
        .map(|(secret_id, wallet_address, agent_key)| AccountProfile {
            secret_id: (*secret_id).to_string(),
            name: (*secret_id).to_string(),
            wallet_address: (*wallet_address).to_string(),
            agent_key: (*agent_key).to_string().into(),
            hydromancer_api_key: "".to_string().into(),
        })
        .collect();
    let payload = SecretPayload::from_credentials(&accounts, "", "");

    KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        encrypted_secrets: Some(encrypt_secrets(&payload, password).expect("encrypt fixture")),
        accounts,
        ..Default::default()
    }
}
