use super::{
    Candle, candle_interval_ms, fill_zero_volume_candle_gaps, interval_requires_hydromancer,
    interval_uses_orderbook_ticks, normalize_candles,
};
use std::collections::BTreeMap;

#[test]
fn candle_normalization_sorts_and_keeps_latest_duplicate() {
    let normalized = normalize_candles(vec![
        Candle::test_price(3_000, 30.0),
        Candle::test_price(1_000, 10.0),
        Candle::test_price(3_000, 31.0),
        Candle::test_price(2_000, 20.0),
    ]);

    assert_eq!(
        normalized
            .iter()
            .map(|candle| (candle.open_time, candle.close))
            .collect::<Vec<_>>(),
        vec![(1_000, 10.0), (2_000, 20.0), (3_000, 31.0)]
    );
}

#[test]
fn candle_normalization_drops_malformed_candles() {
    let mut invalid = Candle::test_price(2_000, 20.0);
    invalid.high = 19.0;

    let mut nan_candle = Candle::test_price(3_000, 30.0);
    nan_candle.close = f64::NAN;

    let normalized = normalize_candles(vec![invalid, Candle::test_price(1_000, 10.0), nan_candle]);

    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].open_time, 1_000);
}

#[test]
fn candle_normalization_preserves_last_valid_payload_for_every_duplicate_order() {
    let mut invalid = Candle::test_price(1_000, 99.0);
    invalid.volume = -1.0;
    let fixtures = [
        (
            Candle::test_ohlcv(1_000, 1_100, [10.0, 15.0, 5.0, 12.0], 1.0),
            true,
        ),
        (
            Candle::test_ohlcv(1_000, 1_200, [20.0, 25.0, 15.0, 22.0], 2.0),
            true,
        ),
        (Candle::test_price(2_000, 30.0), true),
        (invalid, false),
        (
            Candle::test_ohlcv(3_000, 3_100, [-2.0, 0.0, -3.0, -1.0], -0.0),
            true,
        ),
    ];
    let fingerprint = |candle: &Candle| {
        (
            candle.open_time,
            candle.close_time,
            [
                candle.open,
                candle.high,
                candle.low,
                candle.close,
                candle.volume,
            ]
            .map(f64::to_bits),
        )
    };

    // A timestamp map is an independent oracle for the last valid input per key.
    // Enumerate all 3,906 sequences up to five entries, including empty/invalid-only
    // inputs, interleaved groups, and repeated updates to the same timestamp.
    for length in 0..=5 {
        for mut permutation in 0..fixtures.len().pow(length) {
            let mut candles = Vec::new();
            let mut expected = BTreeMap::new();
            for _ in 0..length {
                let (candle, valid) = &fixtures[permutation % fixtures.len()];
                permutation /= fixtures.len();
                candles.push(candle.clone());
                if *valid {
                    expected.insert(candle.open_time, fingerprint(candle));
                }
            }
            let normalized = normalize_candles(candles);
            assert_eq!(
                normalized.iter().map(fingerprint).collect::<Vec<_>>(),
                expected.into_values().collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn zero_volume_gap_fill_preserves_chart_timeline() {
    let candles = fill_zero_volume_candle_gaps(
        vec![
            Candle::test_price(60_000, 10.0),
            Candle::test_price(240_000, 13.0),
        ],
        60_000,
    );

    assert_eq!(
        candles
            .iter()
            .map(|candle| (candle.open_time, candle.close, candle.volume))
            .collect::<Vec<_>>(),
        vec![
            (60_000, 10.0, 10.0),
            (120_000, 10.0, 0.0),
            (180_000, 10.0, 0.0),
            (240_000, 13.0, 10.0),
        ]
    );
}

#[test]
fn one_second_candles_are_hydromancer_only() {
    assert_eq!(candle_interval_ms("1s"), Some(1_000));
    assert!(interval_requires_hydromancer("1s"));
    assert!(!interval_requires_hydromancer("1m"));
}

#[test]
fn tick_candles_are_realtime_only() {
    assert_eq!(candle_interval_ms("tick"), None);
    assert!(interval_uses_orderbook_ticks("tick"));
    assert!(!interval_uses_orderbook_ticks("1s"));
}

#[test]
fn calendar_months_are_not_gap_filled_as_fixed_durations() {
    assert_eq!(candle_interval_ms("1M"), None);
}
