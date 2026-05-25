use super::candle_at;
use crate::chart::{CandlestickChart, ChartState};

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

    let Some(range) = chart.visible_candle_range(&state, 400.0) else {
        panic!("range should clamp into candle data");
    };
    let Some(first_x) = chart.timestamp_to_x(1_000, &state, 400.0) else {
        panic!("first candle should have an x coordinate");
    };

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

    let Some(range) = chart.visible_candle_range(&state, 400.0) else {
        panic!("range should clamp into candle data");
    };

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

    let Some(range) = chart.visible_candle_range(&state, 400.0) else {
        panic!("single candle should still be visible");
    };

    assert_eq!(range.first, 0);
    assert_eq!(range.last, 0);
    assert!(chart.visible_price_params(&state, 400.0, 300.0).is_some());
}
