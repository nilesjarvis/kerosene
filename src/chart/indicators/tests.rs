use crate::api::Candle;

use super::*;

#[test]
fn sma_uses_sliding_window() {
    let candles = vec![
        Candle::test_flat(1_000, 10.0),
        Candle::test_flat(2_000, 20.0),
        Candle::test_flat(3_000, 30.0),
        Candle::test_flat(4_000, 40.0),
    ];

    assert_eq!(
        calculate_sma(&candles, 3),
        vec![(3_000, 20.0), (4_000, 30.0)]
    );
}

#[test]
fn ema_seeds_from_initial_sma() {
    let candles = vec![
        Candle::test_flat(1_000, 10.0),
        Candle::test_flat(2_000, 20.0),
        Candle::test_flat(3_000, 30.0),
        Candle::test_flat(4_000, 40.0),
    ];

    assert_eq!(
        calculate_ema(&candles, 3),
        vec![(3_000, 20.0), (4_000, 30.0)]
    );
}

#[test]
fn zero_period_returns_empty_series() {
    assert!(calculate_sma(&[Candle::test_flat(1_000, 10.0)], 0).is_empty());
    assert!(calculate_ema(&[Candle::test_flat(1_000, 10.0)], 0).is_empty());
}
