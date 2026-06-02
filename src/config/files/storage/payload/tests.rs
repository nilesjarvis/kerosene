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

#[test]
fn merge_plaintext_secrets_prefers_existing_payload_values() {
    let config = KeroseneConfig {
        accounts: vec![
            test_profile("one", "agent-one", "profile-hydro"),
            test_profile("two", "agent-two", ""),
        ],
        hydromancer_api_key: "global-hydro".to_string().into(),
        hyperdash_api_key: "global-hyper".to_string().into(),
        x_bearer_token: "global-x".to_string().into(),
        ..KeroseneConfig::default()
    };
    let mut payload = SecretPayload::from_credentials(
        &[test_profile("one", "existing-agent", "")],
        "existing-hydro",
        "",
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
    assert_eq!(payload.global_x_bearer_token(), "global-x");
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
        x_bearer_token: "old-global-x".to_string().into(),
        ..KeroseneConfig::default()
    };
    let payload = SecretPayload::from_credentials(
        &[test_profile("one", "new-agent", "")],
        "new-global-hydro",
        "new-global-hyper",
        "new-global-x",
    );

    apply_secret_payload(&mut config, &payload);

    assert_eq!(config.accounts[0].agent_key.as_str(), "new-agent");
    assert_eq!(config.accounts[1].agent_key.as_str(), "");
    assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
    assert_eq!(config.accounts[1].hydromancer_api_key.as_str(), "");
    assert_eq!(config.hydromancer_api_key.as_str(), "new-global-hydro");
    assert_eq!(config.hyperdash_api_key.as_str(), "new-global-hyper");
    assert_eq!(config.x_bearer_token.as_str(), "new-global-x");
}
