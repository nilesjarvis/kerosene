use super::{
    KeroseneConfig, SavedLayout, default_config_value, json_string, remove_field, value_from_json,
    value_from_str,
};

#[test]
fn symbol_search_sort_mode_round_trips_and_legacy_defaults_relevance() {
    let config = KeroseneConfig {
        symbol_search_sort_mode: "24h_volume".to_string(),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(decoded.symbol_search_sort_mode, "24h_volume");

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "symbol_search_sort_mode",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded_legacy.symbol_search_sort_mode, "relevance");
}

#[test]
fn liquidation_distribution_symbol_round_trips_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        liquidation_distribution_symbol: "BTC".to_string(),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(decoded.liquidation_distribution_symbol, "BTC");

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "liquidation_distribution_symbol",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(decoded_legacy.liquidation_distribution_symbol.is_empty());
}

#[test]
fn saved_layout_liquidation_distribution_symbol_is_optional_and_round_trips() {
    let decoded: SavedLayout = value_from_json(
        serde_json::json!({
            "name": "distribution",
            "liquidation_distribution_symbol": "BTC"
        }),
        "saved layout with distribution symbol should deserialize",
    );
    assert_eq!(
        decoded.liquidation_distribution_symbol.as_deref(),
        Some("BTC")
    );

    let json = json_string(&decoded, "saved layout should serialize");
    assert!(json.contains("liquidation_distribution_symbol"));
    let round_trip: SavedLayout = value_from_str(&json, "saved layout should deserialize");
    assert_eq!(
        round_trip.liquidation_distribution_symbol.as_deref(),
        Some("BTC")
    );

    let legacy: SavedLayout = value_from_json(
        serde_json::json!({
            "name": "legacy"
        }),
        "legacy saved layout should deserialize",
    );
    assert!(legacy.liquidation_distribution_symbol.is_none());
}

#[test]
fn outcome_display_labels_round_trip_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        outcome_display_labels: std::collections::HashMap::from([(
            "#950".to_string(),
            "YES: Will BTC close green?".to_string(),
        )]),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(
        decoded
            .outcome_display_labels
            .get("#950")
            .map(String::as_str),
        Some("YES: Will BTC close green?")
    );

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "outcome_display_labels",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(decoded_legacy.outcome_display_labels.is_empty());
}

#[test]
fn favourite_symbols_round_trip_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        favourite_symbols: vec!["HYPE".to_string(), "@107".to_string()],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(
        decoded.favourite_symbols,
        vec!["HYPE".to_string(), "@107".to_string()]
    );

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "favourite_symbols",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(decoded_legacy.favourite_symbols.is_empty());
}
