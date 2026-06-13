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

fn test_profile_with_wallet(name: &str, wallet_address: &str, agent_key: &str) -> AccountProfile {
    AccountProfile {
        secret_id: name.to_string(),
        name: name.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: agent_key.to_string().into(),
        hydromancer_api_key: String::new().into(),
    }
}

fn no_legacy_global_secrets(
    _hydromancer_api_key: &mut zeroize::Zeroizing<String>,
    _hyperdash_api_key: &mut zeroize::Zeroizing<String>,
    _x_bearer_token: &mut zeroize::Zeroizing<String>,
) -> Result<(), String> {
    Ok(())
}

fn no_pending_profile_cleanup(_secret_id: &str) -> Result<(), String> {
    Ok(())
}

type NoPendingProfileCleanup = fn(&str) -> Result<(), String>;

fn cleanup_hooks<ClearLegacy>(
    clear_legacy_entries: ClearLegacy,
) -> KeychainCleanupHooks<ClearLegacy, NoPendingProfileCleanup>
where
    ClearLegacy: FnMut(&SecretPayload) -> Result<(), String>,
{
    KeychainCleanupHooks {
        clear_legacy_entries,
        clear_pending_profile: no_pending_profile_cleanup,
    }
}

#[test]
fn pending_keychain_delete_cleanup_removes_bundle_profile_before_secret_hydration() {
    let mut config = KeroseneConfig {
        accounts: vec![test_profile("account-a", "")],
        pending_keychain_profile_deletions: vec!["account-b".to_string()],
        ..KeroseneConfig::default()
    };
    let bundle = RefCell::new(SecretPayload::from_credentials(
        &[
            test_profile("account-a", "agent-a"),
            test_profile("account-b", "agent-b"),
        ],
        "",
        "",
        "",
    ));
    let cleaned_profiles = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.borrow().clone())),
        |payload| {
            bundle.replace(payload.clone());
            Ok(())
        },
        KeychainCleanupHooks {
            clear_legacy_entries: |_: &SecretPayload| Ok(()),
            clear_pending_profile: |secret_id: &str| {
                cleaned_profiles.borrow_mut().push(secret_id.to_string());
                bundle.borrow_mut().remove_profile(secret_id);
                Ok(())
            },
        },
        |_| panic!("valid bundle should not read legacy profile keychain entries"),
        no_legacy_global_secrets,
        |_| {},
    );

    assert_eq!(
        cleaned_profiles.borrow().as_slice(),
        ["account-b".to_string()]
    );
    assert!(config.pending_keychain_profile_deletions.is_empty());
    assert_eq!(config.accounts[0].agent_key.as_str(), "agent-a");
    assert_eq!(bundle.borrow().profile_agent_key("account-b"), None);
}

#[test]
fn pending_keychain_delete_cleanup_failure_preserves_intent_and_redacts_warning() {
    let mut config = KeroseneConfig {
        accounts: vec![test_profile("account-a", "")],
        pending_keychain_profile_deletions: vec!["account-b".to_string()],
        ..KeroseneConfig::default()
    };
    let bundle =
        SecretPayload::from_credentials(&[test_profile("account-a", "agent-a")], "", "", "");
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |_| Ok(()),
        KeychainCleanupHooks {
            clear_legacy_entries: |_: &SecretPayload| Ok(()),
            clear_pending_profile: |_secret_id: &str| Err("delete account-b denied".to_string()),
        },
        |_| panic!("valid bundle should not read legacy profile keychain entries"),
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(
        config.pending_keychain_profile_deletions.as_slice(),
        ["account-b"]
    );
    assert_eq!(config.accounts[0].agent_key.as_str(), "agent-a");
    let warning = warnings
        .borrow()
        .iter()
        .find(|warning| warning.contains("Pending OS keychain account deletion cleanup failed"))
        .cloned()
        .expect("pending cleanup failure should warn");
    assert!(warning.contains("<redacted-profile>"));
    assert!(!warning.contains("account-b"));
}

#[test]
fn pending_full_keychain_cleanup_retries_and_clears_intent() {
    let mut config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        accounts: vec![test_profile("account-a", "")],
        pending_keychain_profile_deletions: vec!["account-b".to_string()],
        pending_keychain_cleanup_all: true,
        ..KeroseneConfig::default()
    };
    let cleaned_profiles = RefCell::new(Vec::new());
    let warnings = RefCell::new(Vec::new());

    retry_pending_keychain_cleanup_all(
        &mut config,
        |profiles| {
            cleaned_profiles.replace(
                profiles
                    .iter()
                    .map(|profile| profile.secret_id.clone())
                    .collect(),
            );
            Ok(())
        },
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(
        cleaned_profiles.borrow().as_slice(),
        ["account-a".to_string(), "account-b".to_string()]
    );
    assert!(!config.pending_keychain_cleanup_all);
    assert!(config.pending_keychain_profile_deletions.is_empty());
    assert!(config.secret_cleanup_state_dirty);
    assert!(warnings.borrow().is_empty());
}

#[test]
fn pending_full_keychain_cleanup_failure_preserves_intent_and_redacts_warning() {
    let mut config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        accounts: vec![test_profile("account-a", "")],
        pending_keychain_profile_deletions: vec!["account-b".to_string()],
        pending_keychain_cleanup_all: true,
        ..KeroseneConfig::default()
    };
    let warnings = RefCell::new(Vec::new());

    retry_pending_keychain_cleanup_all(
        &mut config,
        |_| Err("delete account-a and account-b denied".to_string()),
        |warning| warnings.borrow_mut().push(warning),
    );

    assert!(config.pending_keychain_cleanup_all);
    assert_eq!(
        config.pending_keychain_profile_deletions.as_slice(),
        ["account-b"]
    );
    assert!(!config.secret_cleanup_state_dirty);
    let warning = warnings
        .borrow()
        .iter()
        .find(|warning| warning.contains("Pending OS keychain cleanup failed"))
        .cloned()
        .expect("pending cleanup failure should warn");
    assert!(warning.contains("<redacted-profile>"));
    assert!(!warning.contains("account-a"));
    assert!(!warning.contains("account-b"));
}

#[test]
fn pending_full_keychain_cleanup_waits_for_encrypted_credentials() {
    let mut config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        accounts: vec![test_profile("account-a", "plain-agent")],
        pending_keychain_cleanup_all: true,
        ..KeroseneConfig::default()
    };
    let cleanup_called = Cell::new(false);
    let warnings = RefCell::new(Vec::new());

    load_encrypted_config_secrets_with(
        &mut config,
        |_| {
            cleanup_called.set(true);
            Ok(())
        },
        |_| Ok(()),
        |warning| warnings.borrow_mut().push(warning),
    );

    assert!(!cleanup_called.get());
    assert!(config.pending_keychain_cleanup_all);
    assert!(!config.secret_cleanup_state_dirty);
    assert!(config.secret_migration_save_blocked);
    assert!(config.accounts[0].agent_key.is_empty());
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("cleanup was deferred"))
    );
}

#[test]
fn pending_full_keychain_cleanup_waits_for_authenticated_encrypted_unlock() {
    let mut config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        encrypted_secrets: Some(encrypted_secret_fixture()),
        accounts: vec![test_profile("account-a", "plain-agent")],
        pending_keychain_cleanup_all: true,
        ..KeroseneConfig::default()
    };
    let cleanup_called = Cell::new(false);
    let warnings = RefCell::new(Vec::new());

    load_encrypted_config_secrets_with(
        &mut config,
        |_| {
            cleanup_called.set(true);
            Ok(())
        },
        |_| Ok(()),
        |warning| warnings.borrow_mut().push(warning),
    );

    assert!(!cleanup_called.get());
    assert!(config.pending_keychain_cleanup_all);
    assert!(!config.secret_cleanup_state_dirty);
    assert!(!config.secret_migration_save_blocked);
    assert!(config.accounts[0].agent_key.is_empty());
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("until encrypted credentials are unlocked"))
    );
}

#[test]
fn pending_full_keychain_cleanup_waits_for_valid_encrypted_metadata() {
    let mut invalid_encrypted = encrypted_secret_fixture();
    invalid_encrypted.nonce = "not base64!!!!".to_string();
    let mut config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        encrypted_secrets: Some(invalid_encrypted),
        accounts: vec![test_profile("account-a", "plain-agent")],
        pending_keychain_cleanup_all: true,
        ..KeroseneConfig::default()
    };
    let cleanup_called = Cell::new(false);
    let warnings = RefCell::new(Vec::new());

    load_encrypted_config_secrets_with(
        &mut config,
        |_| {
            cleanup_called.set(true);
            Ok(())
        },
        |_| Ok(()),
        |warning| warnings.borrow_mut().push(warning),
    );

    assert!(!cleanup_called.get());
    assert!(config.pending_keychain_cleanup_all);
    assert!(!config.secret_cleanup_state_dirty);
    assert!(config.secret_migration_save_blocked);
    assert!(config.accounts[0].agent_key.is_empty());
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("metadata is invalid"))
    );
}

#[test]
fn successful_pending_profile_cleanup_marks_config_dirty() {
    let mut config = KeroseneConfig {
        accounts: vec![test_profile("account-a", "")],
        pending_keychain_profile_deletions: vec!["account-b".to_string()],
        ..KeroseneConfig::default()
    };

    retry_pending_keychain_profile_deletions(&mut config, |_| Ok(()), |_| {});

    assert!(config.pending_keychain_profile_deletions.is_empty());
    assert!(config.secret_cleanup_state_dirty);
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

    assert!(load_legacy_os_keychain_secrets_with_warnings(
        &mut config,
        |profile| {
            loaded_profiles.push(profile.name.clone());
            profile.agent_key = format!("{}-agent", profile.name).into();
            Ok(())
        },
        no_legacy_global_secrets,
        |warning| secret_warnings.push(warning),
    ));

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
fn legacy_keychain_startup_reads_active_profile_when_global_hydromancer_might_be_legacy() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![test_profile("one", "plain-agent"), test_profile("two", "")],
        ..KeroseneConfig::default()
    };
    let mut load_count = 0;
    let mut secret_warnings = Vec::new();

    assert!(load_legacy_os_keychain_secrets_with_warnings(
        &mut config,
        |_profile| {
            load_count += 1;
            Ok(())
        },
        no_legacy_global_secrets,
        |warning| secret_warnings.push(warning),
    ));

    assert_eq!(load_count, 1);
    assert_eq!(config.accounts[0].agent_key.as_str(), "plain-agent");
    assert!(
        secret_warnings
            .iter()
            .any(|warning| warning.contains("Only the active legacy account key"))
    );
}

#[test]
fn legacy_profile_read_failure_warning_redacts_account_name() {
    let sensitive_name = "accidentally-pasted-token-secret";
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![AccountProfile {
            secret_id: "one".to_string(),
            name: sensitive_name.to_string(),
            wallet_address: String::new(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        }],
        ..KeroseneConfig::default()
    };
    let mut warnings = Vec::new();

    assert!(!load_legacy_os_keychain_secrets_with_warnings(
        &mut config,
        |_profile| Err("keychain denied read".to_string()),
        no_legacy_global_secrets,
        |warning| warnings.push(warning),
    ));

    let warning = warnings
        .iter()
        .find(|warning| warning.contains("legacy account credential read failed"))
        .expect("read failure warning should be emitted");
    assert!(warning.contains("keychain denied read"));
    assert!(!warning.contains(sensitive_name));
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

    assert!(load_legacy_os_keychain_secrets_with_warnings(
        &mut config,
        |_profile| {
            panic!("active plaintext agent key should not read the keychain");
        },
        no_legacy_global_secrets,
        |warning| secret_warnings.push(warning),
    ));

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
        cleanup_hooks(|_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        }),
        |_| {
            legacy_reads.set(legacy_reads.get() + 1);
            Ok(())
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(legacy_reads.get(), 0);
    assert_eq!(stores.get(), 0);
    assert_eq!(cleanups.get(), 0);
    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert!(!config.secret_migration_save_blocked);
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("left unchanged"))
    );
}

#[test]
fn corrupt_bundle_with_plaintext_secrets_blocks_config_save() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![AccountProfile {
            secret_id: "one".to_string(),
            name: "one".to_string(),
            wallet_address: String::new(),
            agent_key: "plain-agent".to_string().into(),
            hydromancer_api_key: "profile-hydro".to_string().into(),
        }],
        hyperdash_api_key: "global-hyper".to_string().into(),
        x_bearer_token: "global-x".to_string().into(),
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
        cleanup_hooks(|_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        }),
        |_| {
            legacy_reads.set(legacy_reads.get() + 1);
            Ok(())
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(legacy_reads.get(), 0);
    assert_eq!(stores.get(), 0);
    assert_eq!(cleanups.get(), 0);
    assert!(config.secret_migration_save_blocked);
    assert_eq!(config.accounts[0].agent_key.as_str(), "plain-agent");
    assert_eq!(config.hydromancer_api_key.as_str(), "profile-hydro");
    assert_eq!(config.hyperdash_api_key.as_str(), "global-hyper");
    assert_eq!(config.x_bearer_token.as_str(), "global-x");
    assert!(warnings.borrow().iter().any(|warning| {
        warning.contains("Credential bundle read failed")
            && warning.contains("config saves are paused")
    }));
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
        cleanup_hooks(|payload| {
            cleaned_profiles.replace(
                payload
                    .profiles
                    .iter()
                    .map(|profile| profile.secret_id.clone())
                    .collect(),
            );
            Ok(())
        }),
        |profile| {
            profile.agent_key = format!("{}-agent", profile.name).into();
            Ok(())
        },
        no_legacy_global_secrets,
        |_| {},
    );

    assert_eq!(&*stored_profiles.borrow(), &vec!["two".to_string()]);
    assert_eq!(&*cleaned_profiles.borrow(), &vec!["two".to_string()]);
    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert_eq!(config.accounts[1].agent_key.as_str(), "two-agent");
}

#[test]
fn legacy_keychain_migration_reads_global_integration_keys_before_cleanup() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![test_profile("one", "")],
        ..KeroseneConfig::default()
    };
    let stored_payload = RefCell::new(None);
    let cleaned_payload = RefCell::new(None);

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(None),
        |payload| {
            stored_payload.replace(Some(payload.clone()));
            Ok(())
        },
        cleanup_hooks(|payload| {
            cleaned_payload.replace(Some(payload.clone()));
            Ok(())
        }),
        |profile| {
            profile.agent_key = "one-agent".to_string().into();
            Ok(())
        },
        |hydromancer_api_key, hyperdash_api_key, x_bearer_token| {
            *hydromancer_api_key = "legacy-hydro".to_string().into();
            *hyperdash_api_key = "legacy-hyper".to_string().into();
            *x_bearer_token = "legacy-x".to_string().into();
            Ok(())
        },
        |_| {},
    );

    let stored_payload = stored_payload
        .borrow()
        .clone()
        .expect("legacy payload should be stored");
    assert_eq!(stored_payload.global_hydromancer_api_key(), "legacy-hydro");
    assert_eq!(stored_payload.global_hyperdash_api_key(), "legacy-hyper");
    assert_eq!(stored_payload.global_x_bearer_token(), "legacy-x");
    let cleaned_payload = cleaned_payload
        .borrow()
        .clone()
        .expect("cleanup should use migrated payload");
    assert_eq!(cleaned_payload.global_hydromancer_api_key(), "legacy-hydro");
    assert_eq!(cleaned_payload.global_hyperdash_api_key(), "legacy-hyper");
    assert_eq!(cleaned_payload.global_x_bearer_token(), "legacy-x");
}

#[test]
fn legacy_keychain_global_read_failure_blocks_store_and_cleanup() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![test_profile("one", "")],
        ..KeroseneConfig::default()
    };
    let stores = Cell::new(0);
    let cleanups = Cell::new(0);
    let profile_reads = Cell::new(0);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(None),
        |_| {
            stores.set(stores.get() + 1);
            Ok(())
        },
        cleanup_hooks(|_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        }),
        |_| {
            profile_reads.set(profile_reads.get() + 1);
            Ok(())
        },
        |_, _, _| Err("keychain denied read".to_string()),
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(stores.get(), 0);
    assert_eq!(cleanups.get(), 0);
    assert_eq!(profile_reads.get(), 0);
    assert!(config.secret_migration_save_blocked);
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("Legacy shared credential read failed"))
    );
}

#[test]
fn legacy_plaintext_hydromancer_conflict_blocks_bundle_migration() {
    let mut config = KeroseneConfig {
        accounts: vec![
            AccountProfile {
                hydromancer_api_key: "profile-hydro-one".to_string().into(),
                ..test_profile("one", "one-agent")
            },
            AccountProfile {
                hydromancer_api_key: "profile-hydro-two".to_string().into(),
                ..test_profile("two", "two-agent")
            },
        ],
        ..KeroseneConfig::default()
    };
    let bundle = SecretPayload::from_credentials(&[], "", "", "");
    let stores = Cell::new(0);
    let cleanups = Cell::new(0);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |_| {
            stores.set(stores.get() + 1);
            Ok(())
        },
        cleanup_hooks(|_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        }),
        |_| {
            panic!("conflicting plaintext Hydromancer keys should block before keychain reads");
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(stores.get(), 0);
    assert_eq!(cleanups.get(), 0);
    assert!(config.secret_migration_save_blocked);
    assert_eq!(
        config.accounts[0].hydromancer_api_key.as_str(),
        "profile-hydro-one"
    );
    assert_eq!(
        config.accounts[1].hydromancer_api_key.as_str(),
        "profile-hydro-two"
    );
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("multiple legacy Hydromancer API keys"))
    );
}

#[test]
fn legacy_plaintext_hydromancer_conflict_blocks_no_bundle_migration() {
    let mut config = KeroseneConfig {
        accounts: vec![
            AccountProfile {
                hydromancer_api_key: "profile-hydro-one".to_string().into(),
                ..test_profile("one", "one-agent")
            },
            AccountProfile {
                hydromancer_api_key: "profile-hydro-two".to_string().into(),
                ..test_profile("two", "two-agent")
            },
        ],
        ..KeroseneConfig::default()
    };
    let stores = Cell::new(0);
    let cleanups = Cell::new(0);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(None),
        |_| {
            stores.set(stores.get() + 1);
            Ok(())
        },
        cleanup_hooks(|_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        }),
        |_| {
            panic!("conflicting plaintext Hydromancer keys should block before keychain reads");
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(stores.get(), 0);
    assert_eq!(cleanups.get(), 0);
    assert!(config.secret_migration_save_blocked);
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("multiple legacy Hydromancer API keys"))
    );
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
        cleanup_hooks(|payload| {
            cleaned_profiles.replace(
                payload
                    .profiles
                    .iter()
                    .map(|profile| profile.secret_id.clone())
                    .collect(),
            );
            Ok(())
        }),
        |_| {
            panic!("valid bundle should not read legacy profile keychain entries");
        },
        no_legacy_global_secrets,
        |_| {},
    );

    assert_eq!(&*cleaned_profiles.borrow(), &vec!["one".to_string()]);
    assert_eq!(config.accounts[0].agent_key.as_str(), "one-agent");
    assert_eq!(config.accounts[1].agent_key.as_str(), "");
}

#[test]
fn valid_partial_bundle_hydrates_missing_active_profile_from_legacy_keychain() {
    let mut config = KeroseneConfig {
        active_account_index: 1,
        accounts: vec![test_profile("one", ""), test_profile("two", "")],
        ..KeroseneConfig::default()
    };
    let bundle = SecretPayload::from_credentials(&[test_profile("one", "one-agent")], "", "", "");
    let stored_payload = RefCell::new(None);
    let cleaned_payload = RefCell::new(None);
    let loaded_profiles = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |payload| {
            stored_payload.replace(Some(payload.clone()));
            Ok(())
        },
        cleanup_hooks(|payload| {
            cleaned_payload.replace(Some(payload.clone()));
            Ok(())
        }),
        |profile| {
            loaded_profiles.borrow_mut().push(profile.secret_id.clone());
            if profile.secret_id == "two" {
                profile.agent_key = "two-agent".to_string().into();
            }
            Ok(())
        },
        no_legacy_global_secrets,
        |_| {},
    );

    assert_eq!(loaded_profiles.borrow().as_slice(), ["two"]);
    assert_eq!(config.accounts[0].agent_key.as_str(), "one-agent");
    assert_eq!(config.accounts[1].agent_key.as_str(), "two-agent");

    let stored_payload = stored_payload
        .borrow()
        .clone()
        .expect("active legacy fallback should be stored into the bundle");
    assert_eq!(stored_payload.profile_agent_key("one"), Some("one-agent"));
    assert_eq!(stored_payload.profile_agent_key("two"), Some("two-agent"));

    let cleaned_payload = cleaned_payload
        .borrow()
        .clone()
        .expect("cleanup should use the completed bundle payload");
    assert_eq!(cleaned_payload.profile_agent_key("one"), Some("one-agent"));
    assert_eq!(cleaned_payload.profile_agent_key("two"), Some("two-agent"));
    assert!(!config.secret_migration_save_blocked);
}

#[test]
fn valid_partial_bundle_blocks_when_active_legacy_profile_read_fails() {
    let mut config = KeroseneConfig {
        active_account_index: 1,
        accounts: vec![test_profile("one", ""), test_profile("two", "")],
        ..KeroseneConfig::default()
    };
    let bundle = SecretPayload::from_credentials(&[test_profile("one", "one-agent")], "", "", "");
    let stores = Cell::new(0);
    let cleanups = Cell::new(0);
    let loaded_profiles = RefCell::new(Vec::new());
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |_| {
            stores.set(stores.get() + 1);
            Ok(())
        },
        cleanup_hooks(|_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        }),
        |profile| {
            loaded_profiles.borrow_mut().push(profile.secret_id.clone());
            Err("keychain denied read".to_string())
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(loaded_profiles.borrow().as_slice(), ["two"]);
    assert_eq!(stores.get(), 0);
    assert_eq!(cleanups.get(), 0);
    assert!(config.secret_migration_save_blocked);
    assert_eq!(config.accounts[0].agent_key.as_str(), "one-agent");
    assert_eq!(config.accounts[1].agent_key.as_str(), "");
    assert!(warnings.borrow().iter().any(|warning| {
        warning.contains("Active legacy account credential read failed")
            && warning.contains("config saves are paused")
    }));
}

#[test]
fn valid_bundle_binds_legacy_unbound_profile_key_to_wallet() {
    let current_wallet = "0xabc0000000000000000000000000000000000000";
    let other_wallet = "0xdef0000000000000000000000000000000000000";
    let mut config = KeroseneConfig {
        accounts: vec![test_profile_with_wallet("one", current_wallet, "")],
        ..KeroseneConfig::default()
    };
    let mut bundle = SecretPayload::from_credentials(&[], "", "", "");
    assert!(bundle.upsert_profile_agent_key("one", "legacy-agent"));
    let stored_payload = RefCell::new(None);
    let cleaned_payload = RefCell::new(None);

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |payload| {
            stored_payload.replace(Some(payload.clone()));
            Ok(())
        },
        cleanup_hooks(|payload| {
            cleaned_payload.replace(Some(payload.clone()));
            Ok(())
        }),
        |_| {
            panic!("valid bundle should not read legacy profile keychain entries");
        },
        no_legacy_global_secrets,
        |_| {},
    );

    let stored_payload = stored_payload
        .borrow()
        .clone()
        .expect("legacy binding migration should be stored");
    assert_eq!(
        stored_payload.profile_agent_key_for_wallet("one", current_wallet),
        Some("legacy-agent")
    );
    assert_eq!(
        stored_payload.profile_agent_key_for_wallet("one", other_wallet),
        None
    );
    let cleaned_payload = cleaned_payload
        .borrow()
        .clone()
        .expect("cleanup should use the migrated payload");
    assert_eq!(
        cleaned_payload.profile_agent_key_for_wallet("one", current_wallet),
        Some("legacy-agent")
    );
    assert_eq!(config.accounts[0].agent_key.as_str(), "legacy-agent");
}

#[test]
fn valid_bundle_merge_failure_preserves_plaintext_and_blocks_config_save() {
    let mut config = KeroseneConfig {
        accounts: vec![test_profile("one", "plain-agent"), test_profile("two", "")],
        hydromancer_api_key: "plain-hydro".to_string().into(),
        hyperdash_api_key: "plain-hyper".to_string().into(),
        x_bearer_token: "plain-x".to_string().into(),
        ..KeroseneConfig::default()
    };
    let bundle =
        SecretPayload::from_credentials(&[test_profile("two", "bundle-agent")], "", "", "");
    let attempted_store = RefCell::new(None);
    let cleanups = Cell::new(0);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |payload| {
            attempted_store.replace(Some(payload.clone()));
            Err("keychain denied write".to_string())
        },
        cleanup_hooks(|_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        }),
        |_| {
            panic!("valid bundle should not read legacy profile keychain entries");
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    let attempted_store = attempted_store
        .borrow()
        .clone()
        .expect("merged payload should be attempted");
    assert_eq!(
        attempted_store.profile_agent_key("one"),
        Some("plain-agent")
    );
    assert_eq!(
        attempted_store.profile_agent_key("two"),
        Some("bundle-agent")
    );
    assert_eq!(attempted_store.global_hydromancer_api_key(), "plain-hydro");
    assert_eq!(attempted_store.global_hyperdash_api_key(), "plain-hyper");
    assert_eq!(attempted_store.global_x_bearer_token(), "plain-x");

    assert_eq!(cleanups.get(), 0);
    assert!(config.secret_migration_save_blocked);
    assert_eq!(config.accounts[0].agent_key.as_str(), "plain-agent");
    assert_eq!(config.accounts[1].agent_key.as_str(), "bundle-agent");
    assert_eq!(config.hydromancer_api_key.as_str(), "plain-hydro");
    assert_eq!(config.hyperdash_api_key.as_str(), "plain-hyper");
    assert_eq!(config.x_bearer_token.as_str(), "plain-x");
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("config saves are paused"))
    );
}

#[test]
fn valid_bundle_merge_replaces_stale_wallet_bound_agent_key_with_plaintext() {
    let current_wallet = "0xdef0000000000000000000000000000000000000";
    let old_wallet = "0xabc0000000000000000000000000000000000000";
    let mut config = KeroseneConfig {
        accounts: vec![test_profile_with_wallet(
            "one",
            current_wallet,
            "current-agent",
        )],
        ..KeroseneConfig::default()
    };
    let bundle = SecretPayload::from_credentials(
        &[test_profile_with_wallet("one", old_wallet, "stale-agent")],
        "",
        "",
        "",
    );
    let stored_payload = RefCell::new(None);
    let cleaned_payload = RefCell::new(None);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |payload| {
            stored_payload.replace(Some(payload.clone()));
            Ok(())
        },
        cleanup_hooks(|payload| {
            cleaned_payload.replace(Some(payload.clone()));
            Ok(())
        }),
        |_| {
            panic!("valid bundle should not read legacy profile keychain entries");
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    let stored_payload = stored_payload
        .borrow()
        .clone()
        .expect("wallet-aware plaintext merge should be stored");
    assert_eq!(
        stored_payload.profile_agent_key_for_wallet("one", current_wallet),
        Some("current-agent")
    );
    assert_eq!(
        stored_payload.profile_agent_key_for_wallet("one", old_wallet),
        None
    );
    let cleaned_payload = cleaned_payload
        .borrow()
        .clone()
        .expect("cleanup should use the merged payload");
    assert_eq!(
        cleaned_payload.profile_agent_key_for_wallet("one", current_wallet),
        Some("current-agent")
    );

    assert_eq!(config.accounts[0].agent_key.as_str(), "current-agent");
    assert!(
        warnings
            .borrow()
            .iter()
            .all(|warning| { !warning.contains("bound to a different wallet address") })
    );
}

#[test]
fn valid_bundle_cleanup_skips_wallet_bound_profile_that_was_not_loaded() {
    let _warning_guard = crate::config::secrets::secret_warning_test_lock();
    let _ = crate::config::secrets::take_secret_warnings();
    let current_wallet = "0xdef0000000000000000000000000000000000000";
    let old_wallet = "0xabc0000000000000000000000000000000000000";
    let mut config = KeroseneConfig {
        accounts: vec![test_profile_with_wallet("one", current_wallet, "")],
        ..KeroseneConfig::default()
    };
    let bundle = SecretPayload::from_credentials(
        &[test_profile_with_wallet("one", old_wallet, "stale-agent")],
        "hydro-secret",
        "",
        "",
    );
    let cleaned_payload = RefCell::new(None);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |_| Ok(()),
        cleanup_hooks(|payload| {
            cleaned_payload.replace(Some(payload.clone()));
            Ok(())
        }),
        |_| {
            panic!("valid bundle should not read legacy profile keychain entries");
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    let cleaned_payload = cleaned_payload
        .borrow()
        .clone()
        .expect("global legacy cleanup should still be attempted");
    assert!(
        cleaned_payload.profiles.is_empty(),
        "skipped wallet-bound profile must not drive legacy profile cleanup"
    );
    assert_eq!(cleaned_payload.global_hydromancer_api_key(), "hydro-secret");
    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert!(warnings.borrow().is_empty());
    assert!(
        crate::config::secrets::take_secret_warnings()
            .iter()
            .any(|warning| {
                warning.contains("bound to a different wallet address")
                    && warning.contains("Re-enter and save credentials")
            })
    );
}

#[test]
fn valid_bundle_cleanup_runs_with_empty_authoritative_globals() {
    let mut config = KeroseneConfig {
        accounts: vec![test_profile("one", "")],
        ..KeroseneConfig::default()
    };
    let bundle =
        SecretPayload::from_credentials(&[test_profile("one", "bundle-agent")], "", "", "");
    let cleaned_payload = RefCell::new(None);

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(Some(bundle.clone())),
        |_| Ok(()),
        cleanup_hooks(|payload| {
            cleaned_payload.replace(Some(payload.clone()));
            Ok(())
        }),
        |_| {
            panic!("valid bundle should not read legacy profile keychain entries");
        },
        no_legacy_global_secrets,
        |_| {},
    );

    let cleaned_payload = cleaned_payload
        .borrow()
        .clone()
        .expect("valid bundle load should still attempt legacy cleanup");
    assert_eq!(
        cleaned_payload.profile_agent_key("one"),
        Some("bundle-agent")
    );
    assert_eq!(cleaned_payload.global_hydromancer_api_key(), "");
    assert_eq!(cleaned_payload.global_hyperdash_api_key(), "");
    assert_eq!(cleaned_payload.global_x_bearer_token(), "");
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
        cleanup_hooks(|_| Err("delete denied".to_string())),
        |profile| {
            profile.agent_key = "one-agent".to_string().into();
            Ok(())
        },
        no_legacy_global_secrets,
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

#[test]
fn legacy_migration_store_failure_blocks_config_save() {
    let mut config = KeroseneConfig {
        active_account_index: 0,
        accounts: vec![test_profile("one", "")],
        ..KeroseneConfig::default()
    };
    let stores = Cell::new(0);
    let cleanups = Cell::new(0);
    let warnings = RefCell::new(Vec::new());

    load_os_keychain_secrets_with(
        &mut config,
        || Ok(None),
        |_| {
            stores.set(stores.get() + 1);
            Err("keychain denied write".to_string())
        },
        cleanup_hooks(|_| {
            cleanups.set(cleanups.get() + 1);
            Ok(())
        }),
        |profile| {
            profile.agent_key = "one-agent".to_string().into();
            Ok(())
        },
        no_legacy_global_secrets,
        |warning| warnings.borrow_mut().push(warning),
    );

    assert_eq!(stores.get(), 1);
    assert_eq!(cleanups.get(), 0);
    assert!(config.secret_migration_save_blocked);
    assert_eq!(config.accounts[0].agent_key.as_str(), "one-agent");
    assert!(
        warnings
            .borrow()
            .iter()
            .any(|warning| warning.contains("config saves are paused"))
    );
}

fn encrypted_secret_fixture() -> crate::config::EncryptedSecretsConfig {
    crate::config::EncryptedSecretsConfig {
        version: 1,
        kdf: crate::config::secrets::SecretKdfConfig {
            algorithm: "argon2id".to_string(),
            salt: "AAAAAAAAAAAAAAAAAAAAAA==".to_string(),
            memory_kib: 64,
            iterations: 1,
            lanes: 1,
        },
        cipher: "xchacha20poly1305".to_string(),
        nonce: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
        ciphertext: "AAAAAAAAAAAAAAAAAAAAAA==".to_string(),
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

    lock_encrypted_config_secrets_with(&mut config, crate::config::secrets::push_secret_warning);

    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
    assert_eq!(config.hydromancer_api_key.as_str(), "");
    assert_eq!(config.hyperdash_api_key.as_str(), "");
    assert_eq!(config.x_bearer_token.as_str(), "");
}

#[test]
fn encrypted_config_with_invalid_blob_locks_plaintext_and_blocks_save() {
    let mut invalid_encrypted = encrypted_secret_fixture();
    invalid_encrypted.nonce = "not base64!!!!".to_string();
    let _warning_guard = crate::config::secrets::secret_warning_test_lock();
    let _ = crate::config::secrets::take_secret_warnings();
    let mut config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        encrypted_secrets: Some(invalid_encrypted),
        accounts: vec![AccountProfile {
            secret_id: String::new(),
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

    lock_encrypted_config_secrets_with(&mut config, crate::config::secrets::push_secret_warning);

    assert!(config.secret_migration_save_blocked);
    assert!(!config.accounts[0].secret_id.is_empty());
    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
    assert_eq!(config.hydromancer_api_key.as_str(), "");
    assert_eq!(config.hyperdash_api_key.as_str(), "");
    assert_eq!(config.x_bearer_token.as_str(), "");
    assert!(
        crate::config::secrets::take_secret_warnings()
            .iter()
            .any(|warning| {
                warning.contains("Encrypted credential storage metadata is invalid")
                    && warning.contains("config saves are paused")
                    && warning.contains("not loaded into the running session")
            })
    );
}

#[test]
fn encrypted_config_without_blob_locks_plaintext_and_blocks_save() {
    let _warning_guard = crate::config::secrets::secret_warning_test_lock();
    let _ = crate::config::secrets::take_secret_warnings();
    let mut config = KeroseneConfig {
        credential_storage_mode: CredentialStorageMode::EncryptedConfig,
        encrypted_secrets: None,
        accounts: vec![AccountProfile {
            secret_id: String::new(),
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

    lock_encrypted_config_secrets_with(&mut config, crate::config::secrets::push_secret_warning);

    assert!(config.secret_migration_save_blocked);
    assert!(!config.accounts[0].secret_id.is_empty());
    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
    assert_eq!(config.hydromancer_api_key.as_str(), "");
    assert_eq!(config.hyperdash_api_key.as_str(), "");
    assert_eq!(config.x_bearer_token.as_str(), "");
    assert!(
        crate::config::secrets::take_secret_warnings()
            .iter()
            .any(|warning| {
                warning.contains("no encrypted credentials are saved")
                    && warning.contains("config saves are paused")
                    && warning.contains("not loaded into the running session")
            })
    );
}
