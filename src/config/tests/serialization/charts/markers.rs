use super::{
    ChartConfig, KeroseneConfig, MacroIndicatorsConfig, json_string, json_value, object_mut,
    value_from_json, value_from_str,
};

#[test]
fn chart_trade_marker_toggle_round_trips_and_legacy_defaults_off() {
    let macro_indicators = MacroIndicatorsConfig {
        show_volume_profile: true,
        show_session_indicator: true,
        ..MacroIndicatorsConfig::default()
    };
    let config = KeroseneConfig {
        charts: vec![ChartConfig {
            id: 7,
            symbol: "BTC".to_string(),
            timeframe: "H1".to_string(),
            annotations: Vec::new(),
            inverted: false,
            show_trade_markers: true,
            show_earnings_markers: true,
            header_collapsed: true,
            funding_panel_height: 56,
            macro_indicators,
            open_interest_as_notional: true,
            asset_volume_as_notional: false,
            outcome_volume_as_notional: true,
        }],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");

    assert!(decoded.charts[0].show_trade_markers);
    assert!(decoded.charts[0].show_earnings_markers);
    assert!(decoded.charts[0].header_collapsed);
    assert!(decoded.charts[0].open_interest_as_notional);
    assert!(!decoded.charts[0].asset_volume_as_notional);
    assert!(decoded.charts[0].outcome_volume_as_notional);
    assert!(decoded.charts[0].macro_indicators.show_volume_profile);
    assert!(decoded.charts[0].macro_indicators.show_session_indicator);

    let mut legacy_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut legacy_chart, "chart config is an object").remove("show_trade_markers");
    let decoded_chart: ChartConfig =
        value_from_json(legacy_chart, "legacy chart config should deserialize");

    assert!(!decoded_chart.show_trade_markers);
    assert!(decoded_chart.show_earnings_markers);
    assert!(decoded_chart.header_collapsed);
    assert!(decoded_chart.open_interest_as_notional);
    assert!(!decoded_chart.asset_volume_as_notional);

    let mut legacy_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut legacy_chart, "chart config is an object").remove("show_earnings_markers");
    let decoded_chart: ChartConfig =
        value_from_json(legacy_chart, "legacy chart config should deserialize");

    assert!(!decoded_chart.show_earnings_markers);

    let mut legacy_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut legacy_chart, "chart config is an object").remove("header_collapsed");
    let decoded_chart: ChartConfig =
        value_from_json(legacy_chart, "legacy chart config should deserialize");

    assert!(!decoded_chart.header_collapsed);

    let mut older_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut older_chart, "chart config is an object").remove("open_interest_as_notional");
    let decoded_older_chart: ChartConfig =
        value_from_json(older_chart, "older chart config should deserialize");

    assert!(!decoded_older_chart.open_interest_as_notional);

    let mut older_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut older_chart, "chart config is an object").remove("asset_volume_as_notional");
    let decoded_older_chart: ChartConfig =
        value_from_json(older_chart, "older chart config should deserialize");

    assert!(decoded_older_chart.asset_volume_as_notional);

    let mut older_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut older_chart, "chart config is an object").remove("outcome_volume_as_notional");
    let decoded_older_chart: ChartConfig =
        value_from_json(older_chart, "older chart config should deserialize");

    assert!(!decoded_older_chart.outcome_volume_as_notional);

    let mut legacy_macro = json_value(
        &config.charts[0].macro_indicators,
        "macro indicators serialize",
    );
    object_mut(&mut legacy_macro, "macro indicators config is an object")
        .remove("show_volume_profile");
    let decoded_macro: MacroIndicatorsConfig =
        value_from_json(legacy_macro, "legacy macro indicators should deserialize");

    assert!(!decoded_macro.show_volume_profile);

    let mut legacy_macro = json_value(
        &config.charts[0].macro_indicators,
        "macro indicators serialize",
    );
    object_mut(&mut legacy_macro, "macro indicators config is an object")
        .remove("show_session_indicator");
    let decoded_macro: MacroIndicatorsConfig =
        value_from_json(legacy_macro, "legacy macro indicators should deserialize");

    assert!(!decoded_macro.show_session_indicator);
}
