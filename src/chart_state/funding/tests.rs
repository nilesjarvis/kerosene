use super::planning::{funding_attempt_allowed, funding_incremental_due, funding_time_range};
use crate::api::Candle;

fn candle(open_time: u64) -> Candle {
    Candle::test_ohlcv(open_time, open_time, [1.0, 1.0, 1.0, 1.0], 1.0)
}

#[test]
fn funding_range_uses_first_candle_and_caps_end_at_now() {
    let candles = [candle(1_000), candle(2_000)];

    assert_eq!(
        funding_time_range(&candles, 3_600_000, 3_000),
        Some((1_000, 3_000))
    );
}

#[test]
fn funding_range_waits_without_candles_or_duration() {
    assert_eq!(funding_time_range(&[], 3_600_000, 3_000), None);
    assert_eq!(funding_time_range(&[candle(1_000)], 0, 1_000), None);
}

#[test]
fn incremental_funding_waits_until_next_hourly_bucket() {
    assert!(!funding_incremental_due(1_000, 3_600_999));
    assert!(funding_incremental_due(1_000, 3_601_000));
}

#[test]
fn funding_attempts_are_throttled() {
    assert!(funding_attempt_allowed(None, 10_000, 5_000));
    assert!(!funding_attempt_allowed(Some(8_000), 10_000, 5_000));
    assert!(funding_attempt_allowed(Some(5_000), 10_000, 5_000));
}
