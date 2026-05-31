use super::{
    KeroseneConfig, default_config_value, json_string, remove_field, value_from_json,
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
