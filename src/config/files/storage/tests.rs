use super::*;
use std::cell::{Cell, RefCell};

fn test_profile(name: &str, agent_key: &str) -> AccountProfile {
    AccountProfile {
        secret_id: name.to_string(),
        name: name.to_string(),
        wallet_address: String::new(),
        agent_key: agent_key.to_string().into(),
        hydromancer_api_key: String::new().into(),
    }
}

#[test]
fn legacy_keychain_startup_only_loads_active_missing_agent_key() {
    let mut config = KeroseneConfig {
        active_account_index: 1,
        accounts: vec![test_profile("one", ""), test_profile("two", "")],
        ..KeroseneConfig::default()
    };
    let mut loaded_profiles = Vec::new();
    let mut secret_warnings = Vec::new();

    load_legacy_os_keychain_secrets_with_warnings(
        &mut config,
        |profile| {
            loaded_profiles.push(profile.name.clone());
            profile.agent_key = format!("{}-agent", profile.name).into();
            Ok(())
        },
        |warning| secret_warnings.push(warning),
    );

    assert_eq!(loaded_profiles, vec!["two"]);
    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert_eq!(config.accounts[1].agent_key.as_str(), "two-agent");
    assert!(
        secret_warnings
            .iter()
            .any(|warning| warning.contains("Only the active legacy account key"))
    );
}

#[test]
fn legacy_keychain_startup_skips_keychain_when_active_key_is_plaintext() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![test_profile("one", "plain-agent"), test_profile("two", "")],
        ..KeroseneConfig::default()
    };
    let mut load_count = 0;
    let mut secret_warnings = Vec::new();

    load_legacy_os_keychain_secrets_with_warnings(
        &mut config,
        |_profile| {
            load_count += 1;
            Ok(())
        },
        |warning| secret_warnings.push(warning),
    );

    assert_eq!(load_count, 0);
    assert_eq!(config.accounts[0].agent_key.as_str(), "plain-agent");
    assert!(
        secret_warnings
            .iter()
            .any(|warning| warning.contains("Only the active legacy account key"))
    );
}

#[test]
fn legacy_keychain_startup_preserves_plaintext_integration_keys_without_reads() {
    let mut config = KeroseneConfig {
        accounts: vec![AccountProfile {
            secret_id: "one".to_string(),
            name: "one".to_string(),
            wallet_address: String::new(),
            agent_key: "plain-agent".to_string().into(),
            hydromancer_api_key: "profile-hydro".to_string().into(),
        }],
        hyperdash_api_key: "global-hyper".to_string().into(),
        ..KeroseneConfig::default()
    };
    let mut secret_warnings = Vec::new();

    load_legacy_os_keychain_secrets_with_warnings(
        &mut config,
        |_profile| {
            panic!("active plaintext agent key should not read the keychain");
        },
        |warning| secret_warnings.push(warning),
    );

    assert_eq!(config.hydromancer_api_key.as_str(), "profile-hydro");
    assert_eq!(config.hyperdash_api_key.as_str(), "global-hyper");
    assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
    assert!(secret_warnings.is_empty());
}

#[test]
fn corrupt_bundle_does_not_fall_back_to_legacy_keychain_or_overwrite() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![test_profile("one", "")],
        ..KeroseneConfig::default()
    };
    let legacy_reads = Cell::new(0);
    let stores = Cell::new(0);
    let cleanups = Cell::new(0);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Err("keychain payload parse failed".to_string()),
        |_| {
            stores.set(stores.get() + 1);
            Ok(())
        },
        |_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        },
        |_| {
            legacy_reads.set(legacy_reads.get() + 1);
            Ok(())
        },
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(legacy_reads.get(), 0);
    assert_eq!(stores.get(), 0);
    assert_eq!(cleanups.get(), 0);
    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("left unchanged"))
    );
}

#[test]
fn legacy_keychain_migration_cleans_only_profiles_written_to_bundle() {
    let mut config = KeroseneConfig {
        active_account_index: 1,
        accounts: vec![test_profile("one", ""), test_profile("two", "")],
        ..KeroseneConfig::default()
    };
    let stored_profiles = RefCell::new(Vec::new());
    let cleaned_profiles = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(None),
        |payload| {
            stored_profiles.replace(
                payload
                    .profiles
                    .iter()
                    .map(|profile| profile.secret_id.clone())
                    .collect(),
            );
            Ok(())
        },
        |payload| {
            cleaned_profiles.replace(
                payload
                    .profiles
                    .iter()
                    .map(|profile| profile.secret_id.clone())
                    .collect(),
            );
            Ok(())
        },
        |profile| {
            profile.agent_key = format!("{}-agent", profile.name).into();
            Ok(())
        },
        |_| {},
    );

    assert_eq!(&*stored_profiles.borrow(), &vec!["two".to_string()]);
    assert_eq!(&*cleaned_profiles.borrow(), &vec!["two".to_string()]);
    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert_eq!(config.accounts[1].agent_key.as_str(), "two-agent");
}

#[test]
fn valid_bundle_cleanup_scope_uses_payload_profiles_not_all_config_accounts() {
    let mut config = KeroseneConfig {
        accounts: vec![test_profile("one", ""), test_profile("two", "")],
        ..KeroseneConfig::default()
    };
    let bundle = SecretPayload::from_credentials(&[test_profile("one", "one-agent")], "", "", "");
    let cleaned_profiles = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |_| Ok(()),
        |payload| {
            cleaned_profiles.replace(
                payload
                    .profiles
                    .iter()
                    .map(|profile| profile.secret_id.clone())
                    .collect(),
            );
            Ok(())
        },
        |_| {
            panic!("valid bundle should not read legacy profile keychain entries");
        },
        |_| {},
    );

    assert_eq!(&*cleaned_profiles.borrow(), &vec!["one".to_string()]);
    assert_eq!(config.accounts[0].agent_key.as_str(), "one-agent");
    assert_eq!(config.accounts[1].agent_key.as_str(), "");
}

#[test]
fn legacy_migration_cleanup_failure_is_reported_as_cleanup_warning() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![test_profile("one", "")],
        ..KeroseneConfig::default()
    };
    let stores = Cell::new(0);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(None),
        |_| {
            stores.set(stores.get() + 1);
            Ok(())
        },
        |_| Err("delete denied".to_string()),
        |profile| {
            profile.agent_key = "one-agent".to_string().into();
            Ok(())
        },
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(stores.get(), 1);
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("cleanup failed") && warning.contains("delete denied"))
    );
}

fn encrypted_secret_fixture() -> crate::config::EncryptedSecretsConfig {
    crate::config::EncryptedSecretsConfig {
        version: 1,
        kdf: crate::config::secrets::SecretKdfConfig {
            algorithm: "argon2id".to_string(),
            salt: "test-salt".to_string(),
            memory_kib: 64,
            iterations: 1,
            lanes: 1,
        },
        cipher: "xchacha20poly1305".to_string(),
        nonce: "test-nonce".to_string(),
        ciphertext: "encrypted-payload".to_string(),
    }
}

#[test]
fn encrypted_config_lock_clears_all_plaintext_secret_fields() {
    let mut config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        encrypted_secrets: Some(encrypted_secret_fixture()),
        accounts: vec![AccountProfile {
            secret_id: "one".to_string(),
            name: "one".to_string(),
            wallet_address: String::new(),
            agent_key: "plain-agent".to_string().into(),
            hydromancer_api_key: "profile-hydro".to_string().into(),
        }],
        hydromancer_api_key: "global-hydro".to_string().into(),
        hyperdash_api_key: "global-hyper".to_string().into(),
        x_bearer_token: "x-token".to_string().into(),
        ..KeroseneConfig::default()
    };

    lock_encrypted_config_secrets(&mut config);

    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
    assert_eq!(config.hydromancer_api_key.as_str(), "");
    assert_eq!(config.hyperdash_api_key.as_str(), "");
    assert_eq!(config.x_bearer_token.as_str(), "");
}
