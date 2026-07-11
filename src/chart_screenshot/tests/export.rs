use super::*;
use std::sync::Arc;

#[test]
fn screenshot_state_debug_hides_artifact_identity_and_content() {
    const SYMBOL: &str = "private-screenshot-state-symbol-sentinel";
    const TIMEFRAME: &str = "private-screenshot-state-timeframe-sentinel";
    const FILENAME: &str = "private-screenshot-state-filename-sentinel.png";
    let captured_at = local_time(2026, 5, 11, 15, 30);
    let state = ChartScreenshotState {
        symbol: SYMBOL.to_string(),
        timeframe: TIMEFRAME.to_string(),
        width: 2,
        height: 1,
        rgba: Arc::from(vec![11, 22, 33, 44, 55, 66, 77, 88]),
        png: Arc::from(vec![137, 80, 78, 71, 13, 10, 26, 10]),
        preview_handle: iced::widget::image::Handle::from_rgba(
            2,
            1,
            vec![11, 22, 33, 44, 55, 66, 77, 88],
        ),
        captured_at,
        default_filename: FILENAME.to_string(),
    };

    let rendered = format!("{state:?}");
    let captured_at_debug = format!("{captured_at:?}");

    assert!(rendered.contains("width: 2"), "{rendered}");
    assert!(rendered.contains("height: 1"), "{rendered}");
    assert!(rendered.contains("rgba_len: 8"), "{rendered}");
    assert!(rendered.contains("png_len: 8"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    for hidden in [
        SYMBOL,
        TIMEFRAME,
        FILENAME,
        captured_at_debug.as_str(),
        "[11, 22, 33, 44, 55, 66, 77, 88]",
        "[137, 80, 78, 71, 13, 10, 26, 10]",
    ] {
        assert!(!rendered.contains(hidden), "{hidden} leaked in {rendered}");
    }
}

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
