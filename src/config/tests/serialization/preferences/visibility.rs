use super::{
    KeroseneConfig, default_config_value, json_string, remove_field, value_from_json,
    value_from_str,
};

use std::collections::HashMap;

#[test]
fn hide_pnl_round_trips_and_legacy_defaults_visible() {
    let config = KeroseneConfig {
        hide_pnl: true,
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert!(decoded.hide_pnl);

    let mut legacy = default_config_value();
    remove_field(&mut legacy, "hide_pnl", "config should serialize to object");
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(!decoded_legacy.hide_pnl);
}

#[test]
fn hidden_positions_round_trip_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        hidden_positions_by_account: HashMap::from([(
            "acct-a".to_string(),
            vec!["BTC".to_string(), "xyz:CRCL".to_string()],
        )]),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(
        decoded.hidden_positions_by_account.get("acct-a"),
        Some(&vec!["BTC".to_string(), "xyz:CRCL".to_string()])
    );

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "hidden_positions_by_account",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(decoded_legacy.hidden_positions_by_account.is_empty());
}
