use super::{Candle, fill_zero_volume_candle_gaps, normalize_candles};

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
