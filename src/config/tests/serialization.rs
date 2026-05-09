use super::super::secrets;
use super::super::{
    AccountProfile, CredentialStorageMode, EncryptedSecretsConfig, KeroseneConfig,
    default_market_slippage_pct,
};

#[test]
fn legacy_journal_entries_deserialize_without_account_scope() {
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    let object = value
        .as_object_mut()
        .expect("config should serialize to object");
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

    let config: KeroseneConfig =
        serde_json::from_value(value).expect("legacy journal config should deserialize");

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

    let json = serde_json::to_string(&config).expect("config should serialize");

    assert!(json.contains("encrypted_secrets"));
    assert!(!json.contains("agent-secret"));
    assert!(!json.contains("legacy-agent-secret"));
    assert!(!json.contains("hydro-secret"));
    assert!(!json.contains("hyper-secret"));
}

#[test]
fn legacy_config_without_market_slippage_uses_default() {
    let mut value =
        serde_json::to_value(KeroseneConfig::default()).expect("default config should serialize");
    value
        .as_object_mut()
        .expect("config should serialize to object")
        .remove("market_slippage_pct");

    let config: KeroseneConfig =
        serde_json::from_value(value).expect("legacy config should deserialize");

    assert_eq!(config.market_slippage_pct, default_market_slippage_pct());
}
