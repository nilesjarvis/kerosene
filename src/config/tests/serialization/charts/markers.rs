use super::{
    ChartConfig, KeroseneConfig, MacroIndicatorsConfig, json_string, json_value, object_mut,
    value_from_json, value_from_str,
};

#[test]
fn chart_trade_marker_toggle_round_trips_and_legacy_defaults_off() {
    let macro_indicators = MacroIndicatorsConfig {
        sma_50h: true,
        show_volume_profile: true,
        show_session_indicator: true,
        show_leledc_arrows: true,
        show_leledc_levels: true,
        ..MacroIndicatorsConfig::default()
    };
    let config = KeroseneConfig {
        charts: vec![ChartConfig {
            id: 7,
            symbol: "BTC".to_string(),
            secondary_symbol: Some("ETH".to_string()),
            timeframe: "H1".to_string(),
            annotations: Vec::new(),
            inverted: false,
            show_trade_markers: true,
            show_earnings_markers: true,
            header_collapsed: true,
            drawing_toolbar_collapsed: true,
            funding_panel_height: 56,
            session_panel_height: 72,
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
    assert_eq!(decoded.charts[0].secondary_symbol.as_deref(), Some("ETH"));
    assert!(decoded.charts[0].show_earnings_markers);
    assert!(decoded.charts[0].header_collapsed);
    assert!(decoded.charts[0].drawing_toolbar_collapsed);
    assert_eq!(decoded.charts[0].session_panel_height, 72);
    assert!(decoded.charts[0].open_interest_as_notional);
    assert!(!decoded.charts[0].asset_volume_as_notional);
    assert!(decoded.charts[0].outcome_volume_as_notional);
    assert!(decoded.charts[0].macro_indicators.sma_50h);
    assert!(decoded.charts[0].macro_indicators.show_volume_profile);
    assert!(decoded.charts[0].macro_indicators.show_session_indicator);
    assert!(decoded.charts[0].macro_indicators.show_leledc_arrows);
    assert!(decoded.charts[0].macro_indicators.show_leledc_levels);

    let mut legacy_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut legacy_chart, "chart config is an object").remove("secondary_symbol");
    let decoded_chart: ChartConfig =
        value_from_json(legacy_chart, "legacy chart config should deserialize");

    assert_eq!(decoded_chart.secondary_symbol, None);

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

    let mut legacy_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut legacy_chart, "chart config is an object").remove("drawing_toolbar_collapsed");
    let decoded_chart: ChartConfig =
        value_from_json(legacy_chart, "legacy chart config should deserialize");

    assert!(!decoded_chart.drawing_toolbar_collapsed);

    let mut legacy_chart = json_value(&config.charts[0], "chart serializes");
    object_mut(&mut legacy_chart, "chart config is an object").remove("session_panel_height");
    let decoded_chart: ChartConfig =
        value_from_json(legacy_chart, "legacy chart config should deserialize");

    assert_eq!(decoded_chart.session_panel_height, 56);

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

    let mut legacy_macro = json_value(
        &config.charts[0].macro_indicators,
        "macro indicators serialize",
    );
    object_mut(&mut legacy_macro, "macro indicators config is an object").remove("sma_50h");
    let decoded_macro: MacroIndicatorsConfig =
        value_from_json(legacy_macro, "legacy macro indicators should deserialize");

    assert!(!decoded_macro.sma_50h);

    let mut legacy_macro = json_value(
        &config.charts[0].macro_indicators,
        "macro indicators serialize",
    );
    let legacy_macro_object = object_mut(&mut legacy_macro, "macro indicators config is an object");
    legacy_macro_object.remove("show_leledc_arrows");
    legacy_macro_object.remove("show_leledc_levels");
    let decoded_macro: MacroIndicatorsConfig =
        value_from_json(legacy_macro, "legacy macro indicators should deserialize");

    assert!(!decoded_macro.show_leledc_arrows);
    assert!(!decoded_macro.show_leledc_levels);
}
