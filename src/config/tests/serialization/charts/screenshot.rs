use super::{
    ChartScreenshotSettingsConfig, KeroseneConfig, default_config_value, json_string, remove_field,
    value_from_json, value_from_str,
};

#[test]
fn chart_screenshot_settings_round_trip_and_legacy_defaults_visible() {
    let config = KeroseneConfig {
        chart_screenshot_settings: ChartScreenshotSettingsConfig {
            obscure_position_entry: true,
            hide_positions_and_orders: true,
        },
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert!(decoded.chart_screenshot_settings.obscure_position_entry);
    assert!(decoded.chart_screenshot_settings.hide_positions_and_orders);

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "chart_screenshot_settings",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(
        !decoded_legacy
            .chart_screenshot_settings
            .obscure_position_entry
    );
    assert!(
        !decoded_legacy
            .chart_screenshot_settings
            .hide_positions_and_orders
    );
}
