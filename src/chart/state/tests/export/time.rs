use super::*;

#[test]
fn export_state_reconstructs_time_viewport() {
    let chart = test_chart();
    let chart_w = 800.0 - PRICE_AXIS_WIDTH;
    let viewport = ChartViewport {
        start_time_ms: 60_000,
        end_time_ms: 240_000,
        price_lo: 90.0,
        price_hi: 130.0,
        chart_width: 0.0,
        candle_width: 0.0,
        scroll_offset: 0.0,
        y_auto: true,
        y_scale: 1.0,
        y_offset: 0.0,
        funding_y_scale: 1.0,
        funding_y_offset: 0.0,
    };

    let state = ChartState::for_export_viewport(&chart, Some(viewport), chart_w);
    let left = match chart.x_to_timestamp(0.0, &state, chart_w) {
        Some(timestamp) => timestamp,
        None => panic!("missing left timestamp"),
    };
    let right = match chart.x_to_timestamp(chart_w, &state, chart_w) {
        Some(timestamp) => timestamp,
        None => panic!("missing right timestamp"),
    };

    assert_eq!(left, viewport.start_time_ms);
    assert_eq!(right, viewport.end_time_ms);
}
