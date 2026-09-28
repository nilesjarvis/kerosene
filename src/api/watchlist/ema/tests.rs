use super::*;

fn settings(period: usize) -> LiveWatchlistEmaConfig {
    LiveWatchlistEmaConfig {
        period,
        timeframe: "1m".to_string(),
    }
}

fn candles(prices: &[f64]) -> Vec<Candle> {
    prices
        .iter()
        .enumerate()
        .map(|(index, close)| Candle::test_flat((index as u64 + 1) * 60_000, *close))
        .collect()
}

#[test]
fn ema_distance_uses_chart_ema_and_replaces_forming_close_once() {
    let history = candles(&[10.0, 20.0, 30.0, 40.0]);
    let sample =
        WatchlistEmaSample::from_candles(history.clone(), &settings(3), 250_000).expect("EMA");
    assert_eq!(sample.ema, 30.0);
    let mut updated = history;
    updated[3].close = 50.0;
    let expected_ema = crate::chart::calculate_ema(&updated, 3)
        .last()
        .expect("EMA")
        .1;
    let expected = (50.0 / expected_ema - 1.0) * 100.0;
    let distance = sample.distance(Some(50.0), 250_001).expect("distance");
    assert!((distance - expected).abs() < 1e-10);
    assert_eq!(sample.distance(Some(50.0), 250_002), Some(distance));
}

#[test]
fn ema_seed_candle_uses_sma_weight_and_period_one_tracks_mid() {
    let sample =
        WatchlistEmaSample::from_candles(candles(&[10.0, 20.0, 30.0]), &settings(3), 190_000)
            .expect("EMA");
    assert!((sample.distance(Some(60.0), 190_001).expect("distance") - 100.0).abs() < 1e-10);
    let sample =
        WatchlistEmaSample::from_candles(candles(&[10.0]), &settings(1), 70_000).expect("EMA");
    assert_eq!(sample.distance(Some(42.0), 70_001), Some(0.0));
}

#[test]
fn ema_distance_handles_sign_zero_invalid_inputs_and_candle_rollover() {
    let sample =
        WatchlistEmaSample::from_candles(candles(&[100.0, 100.0, 100.0]), &settings(3), 190_000)
            .expect("EMA");
    for price in [110.0, 90.0, 100.0] {
        let ema = 100.0 + (price - 100.0) / 3.0;
        let expected = (price - ema) / ema * 100.0;
        assert!(
            (sample.distance(Some(price), 190_001).expect("distance") - expected).abs() < 1e-10
        );
    }
    for price in [
        None,
        Some(0.0),
        Some(-1.0),
        Some(f64::NAN),
        Some(f64::INFINITY),
    ] {
        assert_eq!(sample.distance(price, 190_001), None);
    }
    assert_eq!(sample.distance(Some(100.0), 240_000), None);
    let mut old_sample = sample;
    old_sample.close_time = 1_000_000;
    assert_eq!(old_sample.distance(Some(100.0), 310_001), None);
}

#[test]
fn ema_history_is_normalized_and_requires_enough_fresh_valid_candles() {
    let mut history = candles(&[10.0, 20.0, 30.0]);
    history.reverse();
    history.push(Candle::test_flat(120_000, 20.0));
    assert_eq!(
        WatchlistEmaSample::from_candles(history, &settings(3), 190_000)
            .expect("EMA")
            .ema,
        20.0
    );
    assert!(
        WatchlistEmaSample::from_candles(candles(&[10.0, 20.0]), &settings(3), 130_000).is_err()
    );
    assert!(
        WatchlistEmaSample::from_candles(candles(&[10.0, 20.0, 30.0]), &settings(3), 500_000)
            .is_err()
    );
    assert!(
        WatchlistEmaSample::from_candles(candles(&[10.0, 0.0, 30.0]), &settings(3), 190_000)
            .is_err()
    );
    assert!(
        WatchlistEmaSample::from_candles(candles(&[10.0, 20.0, 30.0]), &settings(3), 130_000)
            .is_err()
    );
    assert!(WatchlistEmaSample::from_candles(candles(&[10.0]), &settings(0), 70_000).is_err());
}
