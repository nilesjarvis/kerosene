use super::*;

#[test]
fn screenshot_export_chart_applies_privacy_settings_without_mutating_live_chart() {
    let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    instance.chart.active_position = Some(PositionOverlay {
        entry_px: 100_000.0,
        szi: 1.0,
        liquidation_px: Some(80_000.0),
    });
    instance.chart.active_orders.push(OrderOverlay {
        coin: "BTC".to_string(),
        limit_px: 101_000.0,
        sz: 0.25,
        is_buy: true,
        oid: 42,
        is_moving: false,
        pending_state: None,
    });
    let settings = crate::config::ChartScreenshotSettingsConfig {
        obscure_position_entry: true,
        hide_positions_and_orders: true,
    };

    let export_chart = chart_for_screenshot_export(&instance, &settings);

    assert!(export_chart.obscure_position_prices);
    assert!(export_chart.hide_positions_and_orders);
    assert!(export_chart.active_position.is_some());
    assert_eq!(export_chart.active_orders.len(), 1);
    assert!(!instance.chart.obscure_position_prices);
    assert!(!instance.chart.hide_positions_and_orders);
}
