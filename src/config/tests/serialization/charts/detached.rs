use super::{
    DetachedChartWindowConfig, KeroseneConfig, default_config_value, json_string, remove_field,
    value_from_json, value_from_str,
};

#[test]
fn detached_chart_windows_round_trip_and_legacy_defaults_empty() {
    let config = KeroseneConfig {
        detached_chart_windows: vec![DetachedChartWindowConfig {
            chart_id: 7,
            width: 1200.0,
            height: 760.0,
            x: Some(1800.0),
            y: Some(40.0),
        }],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");

    assert_eq!(decoded.detached_chart_windows.len(), 1);
    assert_eq!(decoded.detached_chart_windows[0].chart_id, 7);
    assert_eq!(decoded.detached_chart_windows[0].width, 1200.0);
    assert_eq!(decoded.detached_chart_windows[0].x, Some(1800.0));

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "detached_chart_windows",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");

    assert!(decoded_legacy.detached_chart_windows.is_empty());
}
