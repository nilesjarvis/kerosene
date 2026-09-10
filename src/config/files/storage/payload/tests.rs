use super::*;
use crate::config::AccountProfile;

#[test]
fn config_loss_recovery_preserves_subaccount_parent_and_secret_binding() {
    let mut account =
        test_profile_with_wallet("sub", "0xabc0000000000000000000000000000000000000", "agent");
    account.master_address = Some("0xdef0000000000000000000000000000000000000".to_string());
    let payload = SecretPayload::from_credentials(&[account.clone()], "", "");
    let mut config = KeroseneConfig::default();
    assert_eq!(
        recover_accounts_from_secret_payload(&mut config, &payload),
        1
    );
    assert_eq!(config.accounts[0].master_address, account.master_address);
    assert_eq!(config.accounts[0].wallet_address, account.wallet_address);
    assert!(config.accounts[0].agent_key.is_empty());
    apply_secret_payload(&mut config, &payload);
    assert_eq!(config.accounts[0].agent_key.as_str(), "agent");

    config.accounts[0].master_address = None;
    apply_secret_payload(&mut config, &payload);
    assert!(config.accounts[0].agent_key.is_empty());
}

#[test]
fn config_loss_recovery_skips_invalid_subaccount_metadata() {
    let mut account =
        test_profile_with_wallet("sub", "0xabc0000000000000000000000000000000000000", "agent");
    for master in ["", "invalid", account.wallet_address.as_str()] {
        account.master_address = Some(master.to_string());
        let payload = SecretPayload::from_credentials(&[account.clone()], "", "");
        assert_eq!(
            recover_accounts_from_secret_payload(&mut KeroseneConfig::default(), &payload),
            0
        );
    }
    account.master_address = Some("0xdef0000000000000000000000000000000000000".to_string());
    account.wallet_address.clear();
    let payload = SecretPayload::from_credentials(&[account], "", "");
    assert_eq!(
        recover_accounts_from_secret_payload(&mut KeroseneConfig::default(), &payload),
        0
    );
}

fn test_profile(secret_id: &str, agent_key: &str, hydromancer_key: &str) -> AccountProfile {
    AccountProfile {
        master_address: None,
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
        master_address: None,
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
        x_access_token: "x-token".to_string().into(),
        x_oauth_client_id: "x-client".to_string().into(),
        x_refresh_token: "x-refresh".to_string().into(),
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
    assert_eq!(payload.global_x_access_token(), "x-token");
    assert_eq!(payload.global_x_oauth_client_id(), "x-client");
    assert_eq!(payload.global_x_refresh_token(), "x-refresh");
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
        x_access_token: "old-x-token".to_string().into(),
        x_oauth_client_id: "old-x-client".to_string().into(),
        x_refresh_token: "old-x-refresh".to_string().into(),
        openrouter_api_key: "old-openrouter".to_string().into(),
        ..KeroseneConfig::default()
    };
    let payload = SecretPayload::from_credentials_with_integrations(
        &[test_profile("one", "new-agent", "")],
        "new-global-hydro",
        "new-global-hyper",
        "new-x-token",
        "new-x-client",
        "new-x-refresh",
        "new-openrouter",
    );

    apply_secret_payload(&mut config, &payload);

    assert_eq!(config.accounts[0].agent_key.as_str(), "new-agent");
    assert_eq!(config.accounts[1].agent_key.as_str(), "");
    assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
    assert_eq!(config.accounts[1].hydromancer_api_key.as_str(), "");
    assert_eq!(config.hydromancer_api_key.as_str(), "new-global-hydro");
    assert_eq!(config.hyperdash_api_key.as_str(), "new-global-hyper");
    assert_eq!(config.x_access_token.as_str(), "new-x-token");
    assert_eq!(config.x_oauth_client_id.as_str(), "new-x-client");
    assert_eq!(config.x_refresh_token.as_str(), "new-x-refresh");
    assert_eq!(config.openrouter_api_key.as_str(), "new-openrouter");
}

#[test]
fn apply_secret_payload_preserving_plaintext_only_replaces_present_x_credentials() {
    let mut config = KeroseneConfig {
        x_access_token: "old-x-token".to_string().into(),
        x_oauth_client_id: "old-x-client".to_string().into(),
        x_refresh_token: "old-x-refresh".to_string().into(),
        ..KeroseneConfig::default()
    };
    let empty_payload = SecretPayload::from_credentials(&[], "", "");

    apply_secret_payload_preserving_missing_plaintext(&mut config, &empty_payload);

    assert_eq!(config.x_access_token.as_str(), "old-x-token");
    assert_eq!(config.x_oauth_client_id.as_str(), "old-x-client");
    assert_eq!(config.x_refresh_token.as_str(), "old-x-refresh");

    let stored_payload = SecretPayload::from_credentials_with_x_oauth(
        &[],
        "",
        "",
        "stored-x-token",
        "stored-x-client",
        "stored-x-refresh",
    );
    apply_secret_payload_preserving_missing_plaintext(&mut config, &stored_payload);

    assert_eq!(config.x_access_token.as_str(), "stored-x-token");
    assert_eq!(config.x_oauth_client_id.as_str(), "stored-x-client");
    assert_eq!(config.x_refresh_token.as_str(), "stored-x-refresh");
}

#[test]
fn apply_secret_payload_preserving_plaintext_only_replaces_present_openrouter_key() {
    let mut config = KeroseneConfig {
        openrouter_api_key: "old-openrouter".to_string().into(),
        ..KeroseneConfig::default()
    };
    let empty_payload = SecretPayload::from_credentials(&[], "", "");

    apply_secret_payload_preserving_missing_plaintext(&mut config, &empty_payload);

    assert_eq!(config.openrouter_api_key.as_str(), "old-openrouter");

    let stored_payload = SecretPayload::from_credentials_with_integrations(
        &[],
        "",
        "",
        "",
        "",
        "",
        "stored-openrouter",
    );
    apply_secret_payload_preserving_missing_plaintext(&mut config, &stored_payload);

    assert_eq!(config.openrouter_api_key.as_str(), "stored-openrouter");
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

#[test]
fn proxy_urls_are_hydrated_from_the_secret_bundle() {
    let urls = vec![
        crate::api::proxy::ProxyUrl::parse("https://sentinel:password@proxy.test").expect("URL"),
    ];
    let payload = SecretPayload::default().with_hyperliquid_proxies(&urls);
    let mut config = KeroseneConfig::default();
    apply_secret_payload(&mut config, &payload);
    assert_eq!(config.hyperliquid_proxy_urls, urls);
    assert!(
        !serde_json::to_string(&config)
            .expect("config")
            .contains("sentinel")
    );
    apply_secret_payload(&mut config, &SecretPayload::default());
    assert!(config.hyperliquid_proxy_urls.is_empty());
    apply_secret_payload_preserving_missing_plaintext(&mut config, &payload);
    assert_eq!(config.hyperliquid_proxy_urls, urls);
}
