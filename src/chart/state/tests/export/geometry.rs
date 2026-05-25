use super::*;

#[test]
fn export_state_preserves_right_empty_space_from_geometry() {
    let chart = test_chart();
    let source_chart_w = 500.0;
    let target_chart_w = 1000.0;
    let viewport = ChartViewport {
        start_time_ms: 180_000,
        end_time_ms: 420_000,
        price_lo: 90.0,
        price_hi: 130.0,
        chart_width: source_chart_w,
        candle_width: 20.0,
        scroll_offset: -4.0,
        y_auto: false,
        y_scale: 1.75,
        y_offset: 2.0,
        funding_y_scale: 0.8,
        funding_y_offset: -0.001,
    };

    let state = ChartState::for_export_viewport(&chart, Some(viewport), target_chart_w);

    assert_eq!(state.scroll_offset, viewport.scroll_offset);
    assert_eq!(state.candle_width, 40.0);
    assert!(!state.y_auto);
    assert_eq!(state.y_scale, viewport.y_scale);
    assert_eq!(state.y_offset, viewport.y_offset);

    let Some(last_candle) = chart.candles.last() else {
        panic!("missing last candle");
    };
    let last_x = match chart.timestamp_to_x(last_candle.open_time, &state, target_chart_w) {
        Some(x) => x,
        None => panic!("missing last candle x"),
    };
    let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);

    assert!((target_chart_w - last_x - step * 4.5).abs() < 0.0001);
}
