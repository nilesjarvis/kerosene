use super::*;
use crate::config::AccountProfile;

fn test_profile(secret_id: &str, agent_key: &str, hydromancer_key: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: secret_id.to_string(),
        wallet_address: String::new(),
        agent_key: agent_key.to_string().into(),
        hydromancer_api_key: hydromancer_key.to_string().into(),
    }
}

fn test_profile_with_wallet(
    secret_id: &str,
    wallet_address: &str,
    agent_key: &str,
) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: secret_id.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: agent_key.to_string().into(),
        hydromancer_api_key: String::new().into(),
    }
}

#[test]
fn merge_plaintext_secrets_prefers_existing_payload_values() {
    let config = KeroseneConfig {
        accounts: vec![
            test_profile("one", "agent-one", "profile-hydro"),
            test_profile("two", "agent-two", ""),
        ],
        hydromancer_api_key: "global-hydro".to_string().into(),
        hyperdash_api_key: "global-hyper".to_string().into(),
        ..KeroseneConfig::default()
    };
    let mut payload = SecretPayload::from_credentials(
        &[test_profile("one", "existing-agent", "")],
        "existing-hydro",
        "",
    );

    assert!(merge_missing_plaintext_secrets_into_payload(
        &config,
        &mut payload
    ));

    assert_eq!(payload.profile_agent_key("one"), Some("existing-agent"));
    assert_eq!(payload.profile_agent_key("two"), Some("agent-two"));
    assert_eq!(payload.global_hydromancer_api_key(), "existing-hydro");
    assert_eq!(payload.global_hyperdash_api_key(), "global-hyper");
}

#[test]
fn merge_plaintext_profile_key_records_wallet_binding() {
    let config = KeroseneConfig {
        accounts: vec![test_profile_with_wallet(
            "one",
            "0xABC0000000000000000000000000000000000000",
            "agent-one",
        )],
        ..KeroseneConfig::default()
    };
    let mut payload = SecretPayload::from_credentials(&[], "", "");

    assert!(merge_missing_plaintext_secrets_into_payload(
        &config,
        &mut payload
    ));

    assert_eq!(
        payload.profile_agent_key_for_wallet("one", "0xabc0000000000000000000000000000000000000"),
        Some("agent-one")
    );
    assert_eq!(
        payload.profile_agent_key_for_wallet("one", "0xdef0000000000000000000000000000000000000"),
        None
    );
}

#[test]
fn merge_plaintext_profile_key_replaces_mismatched_wallet_binding() {
    let current_wallet = "0xdef0000000000000000000000000000000000000";
    let old_wallet = "0xabc0000000000000000000000000000000000000";
    let config = KeroseneConfig {
        accounts: vec![test_profile_with_wallet(
            "one",
            current_wallet,
            "current-agent",
        )],
        ..KeroseneConfig::default()
    };
    let mut payload = SecretPayload::from_credentials(
        &[test_profile_with_wallet("one", old_wallet, "stale-agent")],
        "",
        "",
    );

    assert!(merge_missing_plaintext_secrets_into_payload(
        &config,
        &mut payload
    ));

    assert_eq!(
        payload.profile_agent_key_for_wallet("one", current_wallet),
        Some("current-agent")
    );
    assert_eq!(
        payload.profile_agent_key_for_wallet("one", old_wallet),
        None
    );
}

#[test]
fn apply_secret_payload_replaces_plaintext_and_clears_profile_integrations() {
    let mut config = KeroseneConfig {
        accounts: vec![
            test_profile("one", "old-agent", "old-profile-hydro"),
            test_profile("two", "old-agent-two", "old-profile-hydro-two"),
        ],
        hydromancer_api_key: "old-global-hydro".to_string().into(),
        hyperdash_api_key: "old-global-hyper".to_string().into(),
        ..KeroseneConfig::default()
    };
    let payload = SecretPayload::from_credentials(
        &[test_profile("one", "new-agent", "")],
        "new-global-hydro",
        "new-global-hyper",
    );

    apply_secret_payload(&mut config, &payload);

    assert_eq!(config.accounts[0].agent_key.as_str(), "new-agent");
    assert_eq!(config.accounts[1].agent_key.as_str(), "");
    assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
    assert_eq!(config.accounts[1].hydromancer_api_key.as_str(), "");
    assert_eq!(config.hydromancer_api_key.as_str(), "new-global-hydro");
    assert_eq!(config.hyperdash_api_key.as_str(), "new-global-hyper");
}

#[test]
fn apply_secret_payload_skips_wallet_binding_mismatch() {
    let _warning_guard = crate::config::secrets::secret_warning_test_lock();
    let _ = crate::config::take_secret_warnings();
    let sensitive_name = "accidentally-pasted-token-secret";
    let mut config = KeroseneConfig {
        accounts: vec![AccountProfile {
            name: sensitive_name.to_string(),
            ..test_profile_with_wallet(
                "one",
                "0xdef0000000000000000000000000000000000000",
                "old-agent",
            )
        }],
        ..KeroseneConfig::default()
    };
    let payload = SecretPayload::from_credentials(
        &[test_profile_with_wallet(
            "one",
            "0xabc0000000000000000000000000000000000000",
            "new-agent",
        )],
        "",
        "",
    );

    apply_secret_payload(&mut config, &payload);

    assert_eq!(config.accounts[0].agent_key.as_str(), "");
    let warnings = crate::config::take_secret_warnings();
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("bound to a different wallet address"))
    );
    assert!(
        warnings
            .iter()
            .all(|warning| !warning.contains(sensitive_name))
    );
}

#[test]
fn apply_secret_payload_preserving_plaintext_warning_redacts_account_name() {
    let _warning_guard = crate::config::secrets::secret_warning_test_lock();
    let _ = crate::config::take_secret_warnings();
    let sensitive_name = "accidentally-pasted-token-secret";
    let mut config = KeroseneConfig {
        accounts: vec![AccountProfile {
            name: sensitive_name.to_string(),
            ..test_profile_with_wallet(
                "one",
                "0xdef0000000000000000000000000000000000000",
                "old-agent",
            )
        }],
        ..KeroseneConfig::default()
    };
    let payload = SecretPayload::from_credentials(
        &[test_profile_with_wallet(
            "one",
            "0xabc0000000000000000000000000000000000000",
            "new-agent",
        )],
        "",
        "",
    );

    apply_secret_payload_preserving_missing_plaintext(&mut config, &payload);

    assert_eq!(config.accounts[0].agent_key.as_str(), "old-agent");
    let warnings = crate::config::take_secret_warnings();
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("bound to a different wallet address"))
    );
    assert!(
        warnings
            .iter()
            .all(|warning| !warning.contains(sensitive_name))
    );
}
