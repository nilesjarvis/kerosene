use super::*;

#[test]
fn export_state_reconstructs_price_viewport() {
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
        funding_y_scale: 0.5,
        funding_y_offset: 0.002,
    };

    let state = ChartState::for_export_viewport(&chart, Some(viewport), chart_w);
    let Some((price_hi, price_range, _)) = chart.visible_price_params(&state, chart_w, 500.0)
    else {
        panic!("price params");
    };

    assert!((price_hi - viewport.price_hi).abs() < 0.0001);
    assert!((price_range - (viewport.price_hi - viewport.price_lo)).abs() < 0.0001);
    assert_eq!(state.funding_y_scale, viewport.funding_y_scale);
    assert_eq!(state.funding_y_offset, viewport.funding_y_offset);
}
