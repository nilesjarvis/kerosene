use super::*;
use crate::chart::state::DragKind;
use iced::Point;

#[test]
fn export_preserves_canvas_geometry_and_epoch_after_pan_zoom_and_reset() {
    let mut chart = test_chart();
    chart.reset_epoch = 7;
    chart.inverted = true;
    let state = ChartState {
        scroll_offset: -4.0,
        candle_width: 20.0,
        y_auto: false,
        y_scale: 1.75,
        y_offset: 2.0,
        funding_y_scale: 0.8,
        funding_y_offset: -0.001,
        hud_follow_price: true,
        reset_epoch_seen: 7,
        cursor_position: Some(Point::new(200.0, 150.0)),
        drag: Some(DragKind::PanX),
        ..ChartState::default()
    };
    let snapshot = state.snapshot_for_export();
    let draw_state = chart.chart_state_for_draw(&snapshot);
    let exported = draw_state.as_ref();
    assert_eq!(exported.reset_epoch_seen, 7);
    assert_eq!(exported.candle_width, 20.0);
    assert_eq!(exported.scroll_offset, -4.0);
    assert_eq!(exported.funding_y_scale, 0.8);
    assert_eq!(exported.funding_y_offset, -0.001);
    assert!(exported.hud_follow_price);
    assert!(exported.cursor_position.is_none());
    assert!(exported.drag.is_none());
    assert!(state.cursor_position.is_some());
    assert!(state.drag.is_some());

    // Different current layouts (including a just-resized pane) must agree
    // with the live chart without scaling candle widths or rebuilding ranges.
    for width in [320.25, 800.0, 1700.5] {
        let chart_w = width - PRICE_AXIS_WIDTH;
        for candle in &chart.candles {
            assert_eq!(
                chart.timestamp_to_x(candle.open_time, &state, chart_w),
                chart.timestamp_to_x(candle.open_time, exported, chart_w),
            );
        }
        assert_eq!(
            chart.visible_price_params(&state, chart_w, 500.5),
            chart.visible_price_params(exported, chart_w, 500.5),
        );
        let last_x = chart
            .timestamp_to_x(420_000, exported, chart_w)
            .expect("last candle");
        let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);
        assert!((chart_w - last_x - step * 4.5).abs() < 0.001);
    }
}

#[test]
fn export_uses_the_same_pending_reset_as_live_draw() {
    let mut chart = test_chart();
    chart.reset_epoch = 8;
    let state = ChartState {
        candle_width: 30.0,
        scroll_offset: 4.0,
        reset_epoch_seen: 7,
        ..ChartState::default()
    };
    let snapshot = state.snapshot_for_export();
    let live = chart.chart_state_for_draw(&state);
    let export = chart.chart_state_for_draw(&snapshot);
    assert_eq!(export.as_ref().reset_epoch_seen, 8);
    assert_eq!(export.as_ref().candle_width, live.as_ref().candle_width);
    assert_eq!(export.as_ref().scroll_offset, live.as_ref().scroll_offset);
}
