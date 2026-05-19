use super::*;

fn candle(low: f64, high: f64, volume: f64) -> Candle {
    Candle {
        open_time: 1_000,
        close_time: 61_000,
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
fn single_candle_distributes_volume_across_overlapped_buckets() {
    let profile =
        calculate_volume_profile(&[candle(100.0, 104.0, 40.0)], 100.0, 104.0, 4).expect("profile");

    assert_eq!(profile.buckets.len(), 4);
    for bucket in profile.buckets {
        assert_close(bucket.volume, 10.0);
    }
    assert_close(profile.max_volume, 10.0);
}

#[test]
fn candles_outside_visible_price_range_contribute_nothing() {
    let profile = calculate_volume_profile(
        &[candle(90.0, 95.0, 40.0), candle(100.0, 102.0, 20.0)],
        100.0,
        104.0,
        4,
    )
    .expect("profile");

    assert_close(
        profile.buckets.iter().map(|bucket| bucket.volume).sum(),
        20.0,
    );
}

#[test]
fn zero_range_candle_assigns_volume_to_matching_bucket() {
    let profile =
        calculate_volume_profile(&[candle(101.5, 101.5, 12.0)], 100.0, 104.0, 4).expect("profile");

    assert_close(profile.buckets[0].volume, 0.0);
    assert_close(profile.buckets[1].volume, 12.0);
    assert_close(profile.buckets[2].volume, 0.0);
    assert_close(profile.buckets[3].volume, 0.0);
}

#[test]
fn invalid_or_non_positive_candles_are_skipped() {
    let invalid = Candle {
        open_time: 1_000,
        close_time: 61_000,
        open: 100.0,
        high: f64::NAN,
        low: 99.0,
        close: 101.0,
        volume: 20.0,
    };
    let zero_volume = candle(100.0, 104.0, 0.0);

    assert_eq!(
        calculate_volume_profile(&[invalid, zero_volume], 100.0, 104.0, 4),
        None
    );
}

#[test]
fn bucket_count_scales_with_chart_height_and_clamps() {
    assert_eq!(volume_profile_bucket_count(0.0), VOLUME_PROFILE_MIN_BUCKETS);
    assert_eq!(
        volume_profile_bucket_count(96.0),
        VOLUME_PROFILE_MIN_BUCKETS
    );
    assert_eq!(volume_profile_bucket_count(480.0), 60);
    assert_eq!(
        volume_profile_bucket_count(2_000.0),
        VOLUME_PROFILE_MAX_BUCKETS
    );
}
