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
        x_access_token: "x-token".to_string().into(),
        x_oauth_client_id: "x-client".to_string().into(),
        x_refresh_token: "x-refresh".to_string().into(),
        schwab_client_id: "schwab-id".to_string().into(),
        schwab_client_secret: "schwab-secret".to_string().into(),
        schwab_access_token: "schwab-access".to_string().into(),
        schwab_refresh_token: "schwab-refresh".to_string().into(),
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
    assert_eq!(payload.global_schwab_client_id(), "schwab-id");
    assert_eq!(payload.global_schwab_client_secret(), "schwab-secret");
    assert_eq!(payload.global_schwab_access_token(), "schwab-access");
    assert_eq!(payload.global_schwab_refresh_token(), "schwab-refresh");
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
        schwab_client_id: "old-schwab-id".to_string().into(),
        schwab_client_secret: "old-schwab-secret".to_string().into(),
        schwab_access_token: "old-schwab-access".to_string().into(),
        schwab_refresh_token: "old-schwab-refresh".to_string().into(),
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
        "new-schwab-id",
        "new-schwab-secret",
        "new-schwab-access",
        "new-schwab-refresh",
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
    assert_eq!(config.schwab_client_id.as_str(), "new-schwab-id");
    assert_eq!(config.schwab_client_secret.as_str(), "new-schwab-secret");
    assert_eq!(config.schwab_access_token.as_str(), "new-schwab-access");
    assert_eq!(config.schwab_refresh_token.as_str(), "new-schwab-refresh");
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
fn apply_secret_payload_preserving_plaintext_only_replaces_present_schwab_credentials() {
    let mut config = KeroseneConfig {
        schwab_client_id: "old-schwab-id".to_string().into(),
        schwab_client_secret: "old-schwab-secret".to_string().into(),
        schwab_access_token: "old-schwab-access".to_string().into(),
        schwab_refresh_token: "old-schwab-refresh".to_string().into(),
        ..KeroseneConfig::default()
    };
    let empty_payload = SecretPayload::from_credentials(&[], "", "");

    apply_secret_payload_preserving_missing_plaintext(&mut config, &empty_payload);

    assert_eq!(config.schwab_client_id.as_str(), "old-schwab-id");
    assert_eq!(config.schwab_client_secret.as_str(), "old-schwab-secret");
    assert_eq!(config.schwab_access_token.as_str(), "old-schwab-access");
    assert_eq!(config.schwab_refresh_token.as_str(), "old-schwab-refresh");

    let stored_payload = SecretPayload::from_credentials_with_integrations(
        &[],
        "",
        "",
        "",
        "",
        "",
        "stored-schwab-id",
        "stored-schwab-secret",
        "stored-schwab-access",
        "stored-schwab-refresh",
        "",
    );
    apply_secret_payload_preserving_missing_plaintext(&mut config, &stored_payload);

    assert_eq!(config.schwab_client_id.as_str(), "stored-schwab-id");
    assert_eq!(config.schwab_client_secret.as_str(), "stored-schwab-secret");
    assert_eq!(config.schwab_access_token.as_str(), "stored-schwab-access");
    assert_eq!(
        config.schwab_refresh_token.as_str(),
        "stored-schwab-refresh"
    );
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
