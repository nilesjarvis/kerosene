use super::*;

#[test]
fn pnl_card_match_authorization_is_private_and_scoped_to_the_request() {
    let (terminal, _) = TradingTerminal::boot();
    let regular: Value = serde_json::from_slice(
        &terminal
            .build_agent_snapshot_for_request(false)
            .expect("regular snapshot"),
    )
    .expect("regular snapshot json");
    let attached: Value = serde_json::from_slice(
        &terminal
            .build_agent_snapshot_for_request(true)
            .expect("attached snapshot"),
    )
    .expect("attached snapshot json");

    assert_eq!(
        regular["_tool_data"]["assistant_request"]["pnl_card_match_allowed"],
        false
    );
    assert_eq!(
        attached["_tool_data"]["assistant_request"]["pnl_card_match_allowed"],
        true
    );
    assert!(attached.get("assistant_request").is_none());
    assert!(
        attached["data_policy"]["omitted"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "wallet_addresses"))
    );
}

#[test]
fn empty_snapshot_has_versioned_sanitized_contract() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".into());
    terminal.openrouter_api_key = "sk-or-secret".into();
    terminal.hyperdash_api_key = "hyperdash-secret".into();

    let bytes = terminal.build_agent_snapshot().expect("snapshot");
    let text = String::from_utf8(bytes).expect("utf8");
    let value: Value = serde_json::from_str(&text).expect("json");

    assert_eq!(value["schema_version"], SNAPSHOT_SCHEMA_VERSION);
    assert_eq!(value["data_policy"]["access"], "read_only");
    assert_eq!(value["account"]["provenance"]["as_of_ms"], Value::Null);
    assert_eq!(
        value["account"]["provenance"]["observed_at_ms"],
        Value::Null
    );
    assert_eq!(
        value["account"]["provenance"]["freshness"]["state"],
        "unknown"
    );
    assert!(value["account"]["provenance"]["snapshot_generated_at_ms"].is_u64());
    assert_eq!(value["_tool_data"]["markets"]["as_of_ms"], Value::Null);
    assert_eq!(value["_tool_data"]["risk"]["as_of_ms"], Value::Null);
    assert!(!text.contains("0xabc0000000000000000000000000000000000000"));
    assert!(!text.contains("sk-or-secret"));
    assert!(!text.contains("hyperdash-secret"));
    assert_eq!(value["_tool_data"]["contract"]["private"], true);
    assert!(value["_tool_data"]["glossary"]["funding_usdc"].is_string());
}

#[test]
fn provenance_reports_age_and_staleness_without_rewriting_observation_time() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.all_mids.insert("BTC".to_string(), 65_000.0);
    terminal.all_mids_updated_at_ms.insert("BTC".to_string(), 1);

    let bytes = terminal.build_agent_snapshot().expect("snapshot");
    let value: Value = serde_json::from_slice(&bytes).expect("json");
    let provenance = &value["markets"]["provenance"];

    assert_eq!(provenance["observed_at_ms"], 1);
    assert_eq!(provenance["as_of_ms"], 1);
    assert!(
        provenance["age_ms"]
            .as_u64()
            .is_some_and(|age| age > 15_000)
    );
    assert_eq!(provenance["freshness"]["state"], "stale");
    assert_eq!(
        provenance["freshness"]["max_age_ms"],
        ASSISTANT_CURRENT_DATA_MAX_AGE_MS
    );
}
