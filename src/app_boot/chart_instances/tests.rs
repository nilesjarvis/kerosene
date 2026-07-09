use super::*;
use crate::config::MacroIndicatorsConfig;

#[test]
fn boot_chart_instances_restores_trade_marker_toggle() {
    let configs = vec![ChartConfig {
        id: 3,
        symbol: String::new(),
        secondary_symbol: None,
        timeframe: "H1".to_string(),
        annotations: Vec::new(),
        inverted: false,
        show_trade_markers: true,
        show_earnings_markers: true,
        header_collapsed: true,
        drawing_toolbar_collapsed: false,
        funding_panel_height: 56,
        session_panel_height: 72,
        macro_indicators: MacroIndicatorsConfig::default(),
        open_interest_as_notional: false,
        asset_volume_as_notional: false,
        outcome_volume_as_notional: false,
    }];

    let (charts, tasks) = TradingTerminal::boot_chart_instances(
        &configs,
        &std::collections::HashSet::new(),
        crate::config::ChartBackfillSource::Hyperliquid,
        &zeroize::Zeroizing::new(String::new()),
        &zeroize::Zeroizing::new(String::new()),
    );

    assert!(tasks.is_empty());
    assert!(
        charts
            .get(&3)
            .expect("chart instance")
            .chart
            .show_trade_markers
    );
    assert!(
        charts
            .get(&3)
            .expect("chart instance")
            .show_earnings_markers
    );
    assert!(charts.get(&3).expect("chart instance").header_collapsed);
    assert_eq!(
        charts
            .get(&3)
            .expect("chart instance")
            .chart
            .session_panel_height_config(),
        72
    );
    assert!(
        !charts
            .get(&3)
            .expect("chart instance")
            .asset_volume_as_notional
    );
}

#[test]
fn boot_defers_legacy_api_named_spaghetti_series_until_metadata_migration() {
    let mut config = SpaghettiChartConfig::empty(7);
    config.symbols = vec!["@0".to_string()];

    let (charts, tasks) = TradingTerminal::boot_spaghetti_instances(
        &[config],
        &std::collections::HashSet::new(),
        crate::config::ChartBackfillSource::Hyperliquid,
        &zeroize::Zeroizing::new(String::new()),
    );

    assert!(tasks.is_empty(), "raw @0 candle fetch must be deferred");
    let series = &charts[&7].canvas.series[0];
    assert_eq!(series.symbol, "@0");
    assert!(!series.loaded);
    assert!(series.candles.is_empty());
}

#[test]
fn boot_defers_legacy_regular_chart_series_until_metadata_migration() {
    let mut config = ChartConfig::empty(7, "@0", "H1");
    config.secondary_symbol = Some("@0".to_string());

    let (charts, tasks) = TradingTerminal::boot_chart_instances(
        &[config],
        &std::collections::HashSet::new(),
        crate::config::ChartBackfillSource::Hyperliquid,
        &zeroize::Zeroizing::new(String::new()),
        &zeroize::Zeroizing::new(String::new()),
    );

    assert!(
        tasks.is_empty(),
        "raw @0 primary, secondary, and macro candle fetches must be deferred"
    );
    let chart = &charts[&7];
    assert_eq!(chart.symbol, "@0");
    assert_eq!(chart.secondary_symbol.as_deref(), Some("@0"));
    assert!(chart.candle_fetch_request.is_none());
    assert!(chart.secondary_candle_fetch_request.is_none());
    assert_eq!(chart.macro_candles_request_id, 0);
    assert!(chart.chart.candles.is_empty());
    assert!(
        chart
            .chart
            .secondary_series
            .as_ref()
            .is_some_and(|series| series.candles.is_empty())
    );
}
