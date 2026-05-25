use super::*;

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
