use super::*;

fn profile(secret_id: &str, agent_key: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: secret_id.to_string(),
        wallet_address: String::new(),
        agent_key: agent_key.to_string().into(),
        hydromancer_api_key: String::new().into(),
    }
}

#[test]
fn secret_payload_skips_empty_profile_keys() {
    let profiles = vec![profile("acct-a", "agent-a"), profile("acct-b", "")];

    let payload = SecretPayload::from_credentials(&profiles, "", "hyper");

    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
    assert_eq!(payload.profile_agent_key("acct-b"), None);
    assert_eq!(payload.global_hyperdash_api_key(), "hyper");
    assert!(!payload.is_empty());
}

#[test]
fn secret_payload_mutators_keep_bundle_compact() {
    let mut payload = SecretPayload::from_credentials(&[], "", "");

    assert!(payload.is_empty());
    assert!(payload.upsert_profile_agent_key("acct-a", "agent-a"));
    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
    assert!(!payload.upsert_profile_agent_key("acct-a", "agent-a"));
    assert!(payload.upsert_profile_agent_key("acct-a", ""));
    assert_eq!(payload.profile_agent_key("acct-a"), None);
    assert!(payload.set_global_hydromancer_api_key("hydro"));
    assert!(!payload.set_global_hydromancer_api_key("hydro"));
    assert!(!payload.is_empty());
}
