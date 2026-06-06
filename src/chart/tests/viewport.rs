use super::candle_at;
use crate::chart::{CandlestickChart, ChartState};
use crate::config::ChartCrosshairStyle;
use crate::helpers::assert_close_fine as assert_close;

#[test]
fn visible_price_params_reject_invalid_resize_bounds() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 110.0)]);
    let state = ChartState::default();

    assert!(chart.visible_price_params(&state, 0.0, 240.0).is_none());
    assert!(chart.visible_price_params(&state, 400.0, 0.0).is_none());
    assert!(
        chart
            .visible_price_params(&state, f32::NAN, 240.0)
            .is_none()
    );
    assert!(
        chart
            .visible_price_params(&state, 400.0, f32::NAN)
            .is_none()
    );
}

#[test]
fn visible_range_clamps_past_overscroll_to_first_candle() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![
        candle_at(1_000, 10.0),
        candle_at(2_000, 20.0),
        candle_at(3_000, 30.0),
    ]);
    let state = ChartState {
        scroll_offset: 100.0,
        ..ChartState::default()
    };

    let range = chart
        .visible_candle_range(&state, 400.0)
        .expect("range should clamp into candle data");
    let first_x = chart
        .timestamp_to_x(1_000, &state, 400.0)
        .expect("first candle should have an x coordinate");

    assert_eq!(range.first, 0);
    assert_eq!(range.last, 0);
    assert!((0.0..=400.0).contains(&first_x));
}

#[test]
fn visible_range_clamps_future_overscroll_to_latest_candles() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![
        candle_at(1_000, 10.0),
        candle_at(2_000, 20.0),
        candle_at(3_000, 30.0),
    ]);
    let state = ChartState {
        scroll_offset: -1_000.0,
        ..ChartState::default()
    };

    let range = chart
        .visible_candle_range(&state, 400.0)
        .expect("range should clamp into candle data");

    assert!(range.first <= range.last);
    assert_eq!(range.last, 2);
}

#[test]
fn one_candle_visible_range_is_stable() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 10.0)]);
    let state = ChartState {
        scroll_offset: 100.0,
        ..ChartState::default()
    };

    let range = chart
        .visible_candle_range(&state, 400.0)
        .expect("single candle should still be visible");

    assert_eq!(range.first, 0);
    assert_eq!(range.last, 0);
    assert!(chart.visible_price_params(&state, 400.0, 300.0).is_some());
}

#[test]
fn hud_follow_centers_price_window_on_latest_candle_close() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 200.0)]);
    let state = ChartState {
        hud_follow_price: true,
        ..ChartState::default()
    };

    let (price_hi, price_range, _) = chart
        .visible_price_params(&state, 400.0, 300.0)
        .expect("visible price params should be available");

    assert_close(price_hi - price_range * 0.5, 200.0);
}

#[test]
fn hud_follow_keeps_manual_zoom_range_centered_on_latest_close() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 200.0)]);
    let auto_state = ChartState::default();
    let manual_follow_state = ChartState {
        y_auto: false,
        y_scale: 2.0,
        hud_follow_price: true,
        ..ChartState::default()
    };

    let (_, auto_range, _) = chart
        .visible_price_params(&auto_state, 400.0, 300.0)
        .expect("auto price params should be available");
    let (price_hi, price_range, _) = chart
        .visible_price_params(&manual_follow_state, 400.0, 300.0)
        .expect("manual follow price params should be available");

    assert_close(price_range, auto_range * 2.0);
    assert_close(price_hi - price_range * 0.5, 200.0);
}
