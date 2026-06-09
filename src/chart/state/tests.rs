use super::ChartState;
use crate::api::Candle;
use crate::chart::{CANDLE_GAP_RATIO, CandlestickChart, ChartViewport, PRICE_AXIS_WIDTH};
use iced::Point;
use std::time::{Duration, Instant};

mod export;
mod reset;

fn test_chart() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.candles = (0..8)
        .map(|idx| {
            let idx = idx as f64;
            Candle::test_ohlcv(
                idx as u64 * 60_000,
                idx as u64 * 60_000 + 59_999,
                [100.0 + idx, 110.0 + idx, 95.0 + idx, 104.0 + idx],
                1000.0 + idx,
            )
        })
        .collect();
    chart
}

#[test]
fn cursor_speed_sample_tracks_smoothed_pixels_per_second() {
    let mut state = ChartState::default();
    let captured_at = Instant::now();

    state.record_cursor_speed_sample(Some(Point::new(0.0, 0.0)), captured_at);
    state.record_cursor_speed_sample(
        Some(Point::new(100.0, 0.0)),
        captured_at + Duration::from_millis(100),
    );

    assert!((state.hud_cursor_speed_px_per_s - 450.0).abs() < 0.01);
    assert!(state.cursor_speed_sample.is_some());
}

#[test]
fn cursor_speed_sample_clears_when_cursor_leaves_chart() {
    let mut state = ChartState::default();
    let captured_at = Instant::now();

    state.record_cursor_speed_sample(Some(Point::new(0.0, 0.0)), captured_at);
    state.record_cursor_speed_sample(
        Some(Point::new(100.0, 0.0)),
        captured_at + Duration::from_millis(100),
    );
    state.record_cursor_speed_sample(None, captured_at + Duration::from_millis(200));

    assert_eq!(state.hud_cursor_speed_px_per_s, 0.0);
    assert!(state.cursor_speed_sample.is_none());
}
