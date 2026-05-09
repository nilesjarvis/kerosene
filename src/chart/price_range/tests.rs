use crate::api::Candle;

use super::*;

fn candle(low: f64, high: f64, volume: f64) -> Candle {
    Candle {
        open_time: 0,
        close_time: 59_999,
        open: low,
        high,
        low,
        close: high,
        volume,
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn visible_price_stats_include_padding_and_volume_max() {
    let candles = [candle(100.0, 110.0, 5.0), candle(90.0, 120.0, 8.0)];

    let stats = visible_price_stats(&candles, true, 1.0, 0.0).expect("stats");

    assert_close(stats.price_lo, 88.8);
    assert_close(stats.price_hi, 121.2);
    assert_close(stats.price_range, 32.4);
    assert_close(stats.volume_max, 8.0);
}

#[test]
fn visible_price_stats_apply_manual_scale_and_offset() {
    let candles = [candle(100.0, 110.0, 5.0), candle(90.0, 120.0, 8.0)];

    let stats = visible_price_stats(&candles, false, 2.0, 10.0).expect("stats");

    assert_close(stats.price_lo, 82.6);
    assert_close(stats.price_hi, 147.4);
    assert_close(stats.price_range, 64.8);
}

#[test]
fn visible_price_stats_return_none_for_empty_candles() {
    assert_eq!(visible_price_stats(&[], true, 1.0, 0.0), None);
}
