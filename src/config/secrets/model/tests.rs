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

fn profile_with_wallet(secret_id: &str, wallet_address: &str, agent_key: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: secret_id.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: agent_key.to_string().into(),
        hydromancer_api_key: String::new().into(),
    }
}

#[test]
fn secret_payload_skips_empty_profile_keys() {
    let profiles = vec![profile("acct-a", "agent-a"), profile("acct-b", "")];

    let payload = SecretPayload::from_credentials(&profiles, "", "hyper", "x-token");

    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
    assert_eq!(payload.profile_agent_key("acct-b"), None);
    assert_eq!(payload.global_hyperdash_api_key(), "hyper");
    assert_eq!(payload.global_x_bearer_token(), "x-token");
    assert!(!payload.is_empty());
}

#[test]
fn profile_secret_payload_defaults_missing_wallet_binding() {
    let json = r#"{"secret_id":"acct-a","agent_key":"agent-a"}"#;
    let profile: ProfileSecretPayload = serde_json::from_str(json).unwrap();

    assert_eq!(profile.secret_id, "acct-a");
    assert_eq!(profile.wallet_address, None);
    assert_eq!(profile.agent_key.as_str(), "agent-a");
}

#[test]
fn secret_payload_binds_profile_keys_to_normalized_wallet_addresses() {
    let wallet = "0xABC0000000000000000000000000000000000000";
    let profiles = vec![profile_with_wallet("acct-a", wallet, "agent-a")];

    let payload = SecretPayload::from_credentials(&profiles, "", "", "");

    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
    assert_eq!(
        payload
            .profile_agent_key_for_wallet("acct-a", "0xabc0000000000000000000000000000000000000"),
        Some("agent-a")
    );
    assert_eq!(
        payload
            .profile_agent_key_for_wallet("acct-a", "0xdef0000000000000000000000000000000000000"),
        None
    );
    assert!(payload.profile_agent_key_binding_mismatches(
        "acct-a",
        "0xdef0000000000000000000000000000000000000"
    ));
    assert_eq!(
        payload.profiles[0].wallet_address.as_deref(),
        Some("0xabc0000000000000000000000000000000000000")
    );
}

#[test]
fn duplicate_profile_ids_prefer_matching_wallet_binding() {
    let wallet_a = "0xabc0000000000000000000000000000000000000";
    let wallet_b = "0xdef0000000000000000000000000000000000000";
    let wallet_c = "0x9990000000000000000000000000000000000000";
    let profiles = vec![
        profile_with_wallet("acct-a", wallet_a, "agent-a"),
        profile_with_wallet("acct-a", wallet_b, "agent-b"),
    ];

    let payload = SecretPayload::from_credentials(&profiles, "", "", "");

    assert_eq!(
        payload.profile_agent_key_for_wallet("acct-a", wallet_a),
        Some("agent-a")
    );
    assert_eq!(
        payload.profile_agent_key_for_wallet("acct-a", wallet_b),
        Some("agent-b")
    );
    assert!(!payload.profile_agent_key_binding_mismatches("acct-a", wallet_b));
    assert!(payload.profile_agent_key_binding_mismatches("acct-a", wallet_c));
}

#[test]
fn legacy_unbound_profile_keys_still_load_for_any_wallet() {
    let mut payload = SecretPayload::from_credentials(&[], "", "", "");
    assert!(payload.upsert_profile_agent_key("acct-a", "agent-a"));

    assert_eq!(
        payload
            .profile_agent_key_for_wallet("acct-a", "0xabc0000000000000000000000000000000000000"),
        Some("agent-a")
    );
    assert!(!payload.profile_agent_key_binding_mismatches(
        "acct-a",
        "0xdef0000000000000000000000000000000000000"
    ));
}

#[test]
fn legacy_unbound_profile_keys_can_be_bound_to_current_wallet() {
    let current_wallet = "0xabc0000000000000000000000000000000000000";
    let other_wallet = "0xdef0000000000000000000000000000000000000";
    let mut payload = SecretPayload::from_credentials(&[], "", "", "");
    assert!(payload.upsert_profile_agent_key("acct-a", "agent-a"));

    assert!(
        payload.bind_unbound_profile_agent_keys_to_wallets(&[profile_with_wallet(
            "acct-a",
            "0xABC0000000000000000000000000000000000000",
            ""
        )])
    );

    assert_eq!(
        payload.profile_agent_key_for_wallet("acct-a", current_wallet),
        Some("agent-a")
    );
    assert_eq!(
        payload.profile_agent_key_for_wallet("acct-a", other_wallet),
        None
    );
    assert!(payload.profile_agent_key_binding_mismatches("acct-a", other_wallet));
    assert!(
        !payload.bind_unbound_profile_agent_keys_to_wallets(&[profile_with_wallet(
            "acct-a",
            current_wallet,
            ""
        )])
    );
}

#[test]
fn legacy_unbound_profile_binding_skips_invalid_missing_or_ambiguous_wallets() {
    let wallet_a = "0xabc0000000000000000000000000000000000000";
    let wallet_b = "0xdef0000000000000000000000000000000000000";
    let mut payload = SecretPayload::from_credentials(&[], "", "", "");
    assert!(payload.upsert_profile_agent_key("invalid", "invalid-agent"));
    assert!(payload.upsert_profile_agent_key("missing", "missing-agent"));
    assert!(payload.upsert_profile_agent_key("ambiguous", "ambiguous-agent"));

    assert!(!payload.bind_unbound_profile_agent_keys_to_wallets(&[
        profile_with_wallet("invalid", "not-a-wallet", ""),
        profile_with_wallet("ambiguous", wallet_a, ""),
        profile_with_wallet("ambiguous", wallet_b, ""),
    ]));

    for secret_id in ["invalid", "missing", "ambiguous"] {
        let profile = payload
            .profiles
            .iter()
            .find(|profile| profile.secret_id == secret_id)
            .expect("profile should remain");
        assert_eq!(profile.wallet_address, None);
    }
}

#[test]
fn secret_payload_mutators_keep_bundle_compact() {
    let mut payload = SecretPayload::from_credentials(&[], "", "", "");

    assert!(payload.is_empty());
    assert!(payload.upsert_profile_agent_key("acct-a", "agent-a"));
    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
    assert!(!payload.upsert_profile_agent_key("acct-a", "agent-a"));
    assert!(payload.upsert_profile_agent_key("acct-a", ""));
    assert_eq!(payload.profile_agent_key("acct-a"), None);
    assert!(payload.set_global_hydromancer_api_key("hydro"));
    assert!(!payload.set_global_hydromancer_api_key("hydro"));
    assert!(payload.set_global_x_bearer_token("x-token"));
    assert!(!payload.set_global_x_bearer_token("x-token"));
    assert!(!payload.is_empty());
}

#[test]
fn global_secret_payload_defaults_missing_x_token() {
    let json = r#"{"hydromancer_api_key":"hydro","hyperdash_api_key":"hyper"}"#;
    let payload: GlobalSecretPayload = serde_json::from_str(json).unwrap();

    assert_eq!(payload.hydromancer_api_key.as_str(), "hydro");
    assert_eq!(payload.hyperdash_api_key.as_str(), "hyper");
    assert_eq!(payload.x_bearer_token.as_str(), "");
}

#[test]
fn global_secret_payload_defaults_missing_integration_keys() {
    let payload: GlobalSecretPayload = serde_json::from_str(r#"{}"#).unwrap();

    assert_eq!(payload.hydromancer_api_key.as_str(), "");
    assert_eq!(payload.hyperdash_api_key.as_str(), "");
    assert_eq!(payload.x_bearer_token.as_str(), "");

    let payload: GlobalSecretPayload =
        serde_json::from_str(r#"{"hydromancer_api_key":"hydro"}"#).unwrap();
    assert_eq!(payload.hydromancer_api_key.as_str(), "hydro");
    assert_eq!(payload.hyperdash_api_key.as_str(), "");
    assert_eq!(payload.x_bearer_token.as_str(), "");

    let payload: GlobalSecretPayload =
        serde_json::from_str(r#"{"hyperdash_api_key":"hyper"}"#).unwrap();
    assert_eq!(payload.hydromancer_api_key.as_str(), "");
    assert_eq!(payload.hyperdash_api_key.as_str(), "hyper");
    assert_eq!(payload.x_bearer_token.as_str(), "");
}

#[test]
fn secret_payload_defaults_missing_global_bundle() {
    let json = r#"{
        "schema":"kerosene.secrets.v1",
        "profiles":[{"secret_id":"acct-a","agent_key":"agent-a"}]
    }"#;
    let payload: SecretPayload = serde_json::from_str(json).unwrap();

    assert_eq!(payload.schema, SECRET_PAYLOAD_SCHEMA);
    assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
    assert_eq!(payload.global_hydromancer_api_key(), "");
    assert_eq!(payload.global_hyperdash_api_key(), "");
    assert_eq!(payload.global_x_bearer_token(), "");
}

#[test]
fn secret_payload_defaults_missing_profiles_bundle() {
    let json = r#"{
        "schema":"kerosene.secrets.v1",
        "global":{"hydromancer_api_key":"hydro"}
    }"#;
    let payload: SecretPayload = serde_json::from_str(json).unwrap();

    assert_eq!(payload.schema, SECRET_PAYLOAD_SCHEMA);
    assert!(payload.profiles.is_empty());
    assert_eq!(payload.global_hydromancer_api_key(), "hydro");
    assert_eq!(payload.global_hyperdash_api_key(), "");
    assert_eq!(payload.global_x_bearer_token(), "");
}

#[test]
fn secret_payload_debug_redacts_secret_values() {
    let profiles = vec![profile_with_wallet(
        "acct-a",
        "0xABCDEFabcdefABCDEFabcdefABCDEFabcdefabcd",
        "agent-secret",
    )];
    let payload =
        SecretPayload::from_credentials(&profiles, "hydro-secret", "hyper-secret", "x-secret");

    let rendered = format!("{payload:?}");

    assert!(rendered.contains("<redacted>"));
    for secret in [
        "acct-a",
        "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
        "agent-secret",
        "hydro-secret",
        "hyper-secret",
        "x-secret",
    ] {
        assert!(!rendered.contains(secret), "debug output leaked {secret}");
    }
}

#[test]
fn redacted_secret_payload_parse_error_does_not_echo_offending_values() {
    let error = serde_json::from_str::<SecretPayload>(
        r#"{"schema":"kerosene.secrets.v1","profiles":"leaked-agent-key"}"#,
    )
    .expect_err("wrong payload shape should fail");

    let rendered = redacted_secret_payload_parse_error("keychain payload parse failed", error);

    assert!(rendered.contains("keychain payload parse failed"));
    assert!(rendered.contains("secret payload"));
    assert!(rendered.contains("line"));
    assert!(!rendered.contains("leaked-agent-key"));
}

#[test]
fn encrypted_secret_config_debug_redacts_blob_material() {
    let config = EncryptedSecretsConfig {
        version: 1,
        kdf: SecretKdfConfig {
            algorithm: "argon2id".to_string(),
            salt: "salt-secret-material".to_string(),
            memory_kib: 65_536,
            iterations: 3,
            lanes: 1,
        },
        cipher: "xchacha20poly1305".to_string(),
        nonce: "nonce-secret-material".to_string(),
        ciphertext: "ciphertext-secret-material".to_string(),
    };

    let rendered = format!("{config:?}");

    assert!(rendered.contains("argon2id"));
    assert!(rendered.contains("65536"));
    assert!(rendered.contains("<redacted>"));
    for secret in [
        "salt-secret-material",
        "nonce-secret-material",
        "ciphertext-secret-material",
    ] {
        assert!(!rendered.contains(secret), "debug output leaked {secret}");
    }
}
