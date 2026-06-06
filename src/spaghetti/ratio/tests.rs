use super::*;
use crate::helpers::assert_close_tight as assert_close;

fn candle_at(open_time: u64, open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle::test_ohlcv(open_time, open_time + 59_999, [open, high, low, close], 1.0)
}

#[test]
fn ratio_candles_align_by_timestamp_and_use_conservative_high_low() {
    let series_a = vec![
        candle_at(1_000, 10.0, 11.0, 9.0, 10.5),
        candle_at(2_000, 100.0, 120.0, 90.0, 110.0),
    ];
    let series_b = vec![
        candle_at(1_500, 40.0, 44.0, 36.0, 38.0),
        candle_at(2_000, 50.0, 55.0, 40.0, 44.0),
    ];

    let ratios = build_ratio_candles(&series_a, &series_b, 0.0, 3_000.0, &|ts| ts as f32);

    assert_eq!(ratios.len(), 1);
    assert_eq!(ratios[0].x, 2_000.0);
    assert_close(ratios[0].open, 2.0);
    assert_close(ratios[0].high, 3.0);
    assert_close(ratios[0].low, 90.0 / 55.0);
    assert_close(ratios[0].close, 2.5);
    assert!(ratios[0].high >= ratios[0].low);
}

#[test]
fn ratio_candles_ignore_invalid_or_non_positive_prices() {
    let mut bad_a = candle_at(1_000, 10.0, 11.0, 9.0, 10.5);
    bad_a.low = 0.0;
    let series_a = vec![bad_a, candle_at(2_000, 100.0, 120.0, 90.0, 110.0)];
    let series_b = vec![
        candle_at(1_000, 50.0, 55.0, 40.0, 44.0),
        candle_at(2_000, 50.0, 55.0, 40.0, 44.0),
    ];

    let ratios = build_ratio_candles(&series_a, &series_b, 0.0, 3_000.0, &|_| 0.0);

    assert_eq!(ratios.len(), 1);
    assert_close(ratios[0].open, 2.0);
}

#[test]
fn ratio_candles_ignore_points_outside_visible_time_window() {
    let series_a = vec![
        candle_at(1_000, 10.0, 11.0, 9.0, 10.5),
        candle_at(2_000, 100.0, 120.0, 90.0, 110.0),
    ];
    let series_b = vec![
        candle_at(1_000, 5.0, 5.5, 4.0, 4.4),
        candle_at(2_000, 50.0, 55.0, 40.0, 44.0),
    ];

    let ratios = build_ratio_candles(&series_a, &series_b, 1_500.0, 2_500.0, &|_| 0.0);

    assert_eq!(ratios.len(), 1);
    assert_close(ratios[0].open, 2.0);
}

#[test]
fn ratio_candles_return_empty_without_timestamp_overlap() {
    let series_a = vec![candle_at(1_000, 10.0, 11.0, 9.0, 10.5)];
    let series_b = vec![candle_at(2_000, 5.0, 5.5, 4.0, 4.4)];

    let ratios = build_ratio_candles(&series_a, &series_b, 0.0, 3_000.0, &|_| 0.0);

    assert!(ratios.is_empty());
}

#[test]
fn ratio_format_preserves_small_pair_precision() {
    assert_eq!(format_ratio_value(0.00053204), "0.000532");
    assert_eq!(format_ratio_value(0.00001234), "0.00001234");
    assert_eq!(format_ratio_value(f64::NAN), "--");
}

#[test]
fn minimum_ratio_range_scales_with_ratio_magnitude() {
    assert_close(minimum_ratio_range(0.00050, 0.00054), 0.0000108);
    assert_close(minimum_ratio_range(2.0, 2.1), 0.042);
}
