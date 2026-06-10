use super::{
    KeroseneConfig, default_config_value, default_market_slippage_pct, json_string, remove_field,
    value_from_json, value_from_str,
};

#[test]
fn legacy_config_without_market_slippage_uses_default() {
    let mut value = default_config_value();
    remove_field(
        &mut value,
        "market_slippage_pct",
        "config should serialize to object",
    );

    let config: KeroseneConfig = value_from_json(value, "legacy config should deserialize");

    assert_eq!(config.market_slippage_pct, default_market_slippage_pct());
}

#[test]
fn order_quantity_denomination_round_trips_and_legacy_defaults_coin() {
    let config = KeroseneConfig {
        order_quantity_is_usd: true,
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert!(decoded.order_quantity_is_usd);

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "order_quantity_is_usd",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(!decoded_legacy.order_quantity_is_usd);
}

#[test]
fn optimistic_account_updates_round_trips_and_legacy_defaults_off() {
    let config = KeroseneConfig {
        optimistic_account_updates: true,
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert!(decoded.optimistic_account_updates);

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "optimistic_account_updates",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(!decoded_legacy.optimistic_account_updates);
}
