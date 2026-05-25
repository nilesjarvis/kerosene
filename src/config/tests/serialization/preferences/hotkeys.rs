use super::{
    HotkeyPrefixConfig, KeroseneConfig, default_config_value, json_string, remove_field,
    value_from_json, value_from_str,
};

#[test]
fn chart_timeframe_hotkey_prefix_round_trips_and_legacy_defaults_none() {
    let config = KeroseneConfig {
        chart_timeframe_hotkey_prefix: Some(HotkeyPrefixConfig {
            shift: false,
            ctrl: false,
            alt: false,
            logo: true,
        }),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(
        decoded.chart_timeframe_hotkey_prefix,
        config.chart_timeframe_hotkey_prefix
    );

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "chart_timeframe_hotkey_prefix",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded_legacy.chart_timeframe_hotkey_prefix, None);
}
