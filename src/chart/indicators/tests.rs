use crate::api::Candle;

use super::*;

fn candle_at(open_time: u64, close: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: close,
        high: close,
        low: close,
        close,
        volume: 1.0,
    }
}

#[test]
fn sma_uses_sliding_window() {
    let candles = vec![
        candle_at(1_000, 10.0),
        candle_at(2_000, 20.0),
        candle_at(3_000, 30.0),
        candle_at(4_000, 40.0),
    ];

    assert_eq!(
        calculate_sma(&candles, 3),
        vec![(3_000, 20.0), (4_000, 30.0)]
    );
}

#[test]
fn ema_seeds_from_initial_sma() {
    let candles = vec![
        candle_at(1_000, 10.0),
        candle_at(2_000, 20.0),
        candle_at(3_000, 30.0),
        candle_at(4_000, 40.0),
    ];

    assert_eq!(
        calculate_ema(&candles, 3),
        vec![(3_000, 20.0), (4_000, 30.0)]
    );
}

#[test]
fn zero_period_returns_empty_series() {
    assert!(calculate_sma(&[candle_at(1_000, 10.0)], 0).is_empty());
    assert!(calculate_ema(&[candle_at(1_000, 10.0)], 0).is_empty());
}
