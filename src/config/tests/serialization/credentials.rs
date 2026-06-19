use super::super::config_warning_guard;
use super::{default_config_value, json_string, object_mut, value_from_json, value_from_str};
use crate::config::secrets;
use crate::config::{
    AccountProfile, CredentialStorageMode, EncryptedSecretsConfig, KeroseneConfig,
    take_config_warnings,
};

#[test]
fn legacy_journal_entries_deserialize_without_account_scope() {
    let mut value = default_config_value();
    let object = object_mut(&mut value, "config should serialize to object");
    object.remove("journal_entries_by_account");
    object.insert(
        "journal_entries".to_string(),
        serde_json::json!({
            "BTC_1": {
                "open": "legacy note",
                "close": ""
            }
        }),
    );

    let config: KeroseneConfig = value_from_json(value, "legacy journal config should deserialize");

    assert!(config.journal_entries_by_account.is_empty());
    assert_eq!(
        config
            .journal_entries
            .get("BTC_1")
            .map(|entry| entry.open.as_str()),
        Some("legacy note")
    );
}

#[test]
fn serialized_config_keeps_raw_credentials_out_of_json() {
    let profiles = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Main".to_string(),
        wallet_address: String::new(),
        agent_key: "agent-secret".to_string().into(),
        hydromancer_api_key: String::new().into(),
    }];
    let config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        encrypted_secrets: Some(EncryptedSecretsConfig {
            version: 1,
            kdf: secrets::SecretKdfConfig {
                algorithm: "argon2id".to_string(),
                salt: "test-salt".to_string(),
                memory_kib: 64,
                iterations: 1,
                lanes: 1,
            },
            cipher: "xchacha20poly1305".to_string(),
            nonce: "test-nonce".to_string(),
            ciphertext: "encrypted payload".to_string(),
        }),
        accounts: profiles,
        agent_key: "legacy-agent-secret".to_string().into(),
        hydromancer_api_key: "hydro-secret".to_string().into(),
        hyperdash_api_key: "hyper-secret".to_string().into(),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");

    assert!(json.contains("encrypted_secrets"));
    assert!(!json.contains("agent-secret"));
    assert!(!json.contains("legacy-agent-secret"));
    assert!(!json.contains("hydro-secret"));
    assert!(!json.contains("hyper-secret"));
}

#[test]
fn pending_keychain_profile_deletions_default_empty_for_legacy_configs() {
    let mut value = default_config_value();
    object_mut(&mut value, "config should serialize to object")
        .remove("pending_keychain_profile_deletions");
    object_mut(&mut value, "config should serialize to object")
        .remove("pending_keychain_cleanup_all");

    let config: KeroseneConfig = value_from_json(
        value,
        "legacy config without pending deletes should deserialize",
    );

    assert!(config.pending_keychain_profile_deletions.is_empty());
    assert!(!config.pending_keychain_cleanup_all);
}

#[test]
fn pending_keychain_profile_deletions_omitted_when_empty() {
    let json = json_string(
        &KeroseneConfig::default(),
        "default config should serialize",
    );

    assert!(!json.contains("pending_keychain_profile_deletions"));
    assert!(!json.contains("pending_keychain_cleanup_all"));
}

#[test]
fn serialized_pending_keychain_cleanup_intents_do_not_include_raw_credentials() {
    let config = KeroseneConfig {
        pending_keychain_profile_deletions: vec!["acct-a".to_string()],
        pending_keychain_cleanup_all: true,
        accounts: vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Main".to_string(),
            wallet_address: String::new(),
            agent_key: "agent-secret".to_string().into(),
            hydromancer_api_key: "hydro-secret".to_string().into(),
        }],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config with pending delete should serialize");

    assert!(json.contains("pending_keychain_profile_deletions"));
    assert!(json.contains("acct-a"));
    assert!(json.contains("pending_keychain_cleanup_all"));
    assert!(!json.contains("agent-secret"));
    assert!(!json.contains("hydro-secret"));
}

#[test]
fn credential_storage_mode_wire_values_round_trip_and_missing_defaults_to_keychain() {
    assert_eq!(
        json_string(
            &CredentialStorageMode::OsKeychain,
            "OS keychain mode should serialize"
        ),
        "\"OsKeychain\""
    );
    assert_eq!(
        json_string(
            &CredentialStorageMode::EncryptedConfig,
            "encrypted config mode should serialize"
        ),
        "\"EncryptedConfig\""
    );

    let os_keychain: CredentialStorageMode =
        value_from_str("\"OsKeychain\"", "OS keychain mode should deserialize");
    let encrypted_config: CredentialStorageMode = value_from_str(
        "\"EncryptedConfig\"",
        "encrypted config mode should deserialize",
    );
    assert_eq!(os_keychain, CredentialStorageMode::OsKeychain);
    assert_eq!(encrypted_config, CredentialStorageMode::EncryptedConfig);

    let mut value = default_config_value();
    object_mut(&mut value, "config should serialize to object").remove("credential_storage_mode");
    let config: KeroseneConfig = value_from_json(
        value,
        "legacy config without storage mode should deserialize",
    );
    assert_eq!(
        config.credential_storage_mode,
        CredentialStorageMode::OsKeychain
    );
}

#[test]
fn invalid_credential_storage_mode_defaults_to_keychain_with_warning() {
    let _warning_guard = config_warning_guard();
    let mut value = default_config_value();
    object_mut(&mut value, "config should serialize to object").insert(
        "credential_storage_mode".to_string(),
        serde_json::json!("FutureStorageMode"),
    );

    let config: KeroseneConfig = value_from_json(
        value,
        "config with future credential storage mode should deserialize",
    );

    assert_eq!(
        config.credential_storage_mode,
        CredentialStorageMode::OsKeychain
    );

    let warnings = take_config_warnings();
    assert!(
        warnings.iter().any(
            |warning| warning == "Unknown credential storage mode in config; using OS Keychain"
        )
    );
    assert!(
        !warnings
            .iter()
            .any(|warning| warning.contains("FutureStorageMode"))
    );
}
