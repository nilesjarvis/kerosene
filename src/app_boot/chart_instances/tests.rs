use super::*;
use crate::config::MacroIndicatorsConfig;

#[test]
fn boot_chart_instances_restores_trade_marker_toggle() {
    let configs = vec![ChartConfig {
        id: 3,
        symbol: String::new(),
        timeframe: "H1".to_string(),
        annotations: Vec::new(),
        inverted: false,
        show_trade_markers: true,
        show_earnings_markers: true,
        header_collapsed: true,
        funding_panel_height: 56,
        macro_indicators: MacroIndicatorsConfig::default(),
        open_interest_as_notional: false,
        outcome_volume_as_notional: false,
    }];

    let (charts, tasks) = TradingTerminal::boot_chart_instances(
        &configs,
        &std::collections::HashSet::new(),
        crate::config::ChartBackfillSource::Hyperliquid,
        String::new(),
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
}
