use super::super::tests::test_cache_dir;
use super::*;
use std::fs;

#[test]
fn candle_range_load_requires_cached_coverage() {
    let root = test_cache_dir("candles");
    let candles = vec![
        Candle::test_flat(1_000, 100.0),
        Candle::test_flat(61_000, 101.0),
        Candle::test_flat(121_000, 102.0),
    ];
    save_candle_snapshot(
        root.clone(),
        ChartBackfillSource::Hyperliquid,
        "BTC",
        "1m",
        candles,
    )
    .expect("snapshot save succeeds");

    let cached = load_candles_for_range_from_dir(
        &root,
        ChartBackfillSource::Hyperliquid,
        "BTC",
        "1m",
        61_000,
        181_000,
    )
    .expect("range load succeeds")
    .expect("range should be covered");
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].open_time, 61_000);

    let missing = load_candles_for_range_from_dir(
        &root,
        ChartBackfillSource::Hyperliquid,
        "BTC",
        "1m",
        0,
        301_000,
    )
    .expect("range load succeeds");
    assert!(missing.is_none());

    let _ = fs::remove_dir_all(root);
}

/// A snapshot whose stale head is stitched to a fresh tail (the reported
/// 1m HYPE "4d ago -> 3h ago" gap). The recent tail is within lookback, so
/// the old gate would serve the whole gapped vec; the fix serves only the
/// trailing contiguous run.
fn gapped_hype_snapshot(now_ms: u64) -> Vec<Candle> {
    let four_days = 4 * 24 * 60 * 60 * 1_000;
    let old_start = now_ms - four_days;
    let recent_start = now_ms - 180_000;
    vec![
        Candle::test_flat(old_start, 60.0),
        Candle::test_flat(old_start + 60_000, 60.0),
        Candle::test_flat(old_start + 120_000, 60.0),
        Candle::test_flat(recent_start, 70.0),
        Candle::test_flat(recent_start + 60_000, 70.0),
        Candle::test_flat(recent_start + 120_000, 70.0),
    ]
}

#[test]
fn load_fresh_candles_serves_only_trailing_run_across_interior_gap() {
    let root = test_cache_dir("fresh-gap");
    let now_ms = 10_000_000_000;
    save_candle_snapshot(
        root.clone(),
        ChartBackfillSource::Hyperliquid,
        "HYPE",
        "1m",
        gapped_hype_snapshot(now_ms),
    )
    .expect("snapshot save succeeds");

    let served = load_fresh_candles_from_dir(
        &root,
        ChartBackfillSource::Hyperliquid,
        "HYPE",
        Timeframe::M1,
        now_ms,
    )
    .expect("load succeeds")
    .expect("recent tail is fresh");

    // Only the recent contiguous block survives; the phantom $60 head is gone.
    assert_eq!(served.len(), 3);
    assert_eq!(served[0].open_time, now_ms - 180_000);
    assert!(served.iter().all(|candle| candle.close == 70.0));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn sparse_spot_startup_cache_keeps_a_small_legitimate_trade_gap() {
    let root = test_cache_dir("fresh-sparse-spot");
    let now_ms = 10_000_000_000;
    save_candle_snapshot(
        root.clone(),
        ChartBackfillSource::Hyperliquid,
        "@3",
        "1m",
        vec![
            Candle::test_flat(now_ms - 180_000, 100.0),
            Candle::test_flat(now_ms - 60_000, 101.0),
        ],
    )
    .expect("snapshot save succeeds");

    let served = load_fresh_candles_from_dir(
        &root,
        ChartBackfillSource::Hyperliquid,
        "@3",
        Timeframe::M1,
        now_ms,
    )
    .expect("load succeeds")
    .expect("sparse tail is fresh");

    assert_eq!(served.len(), 2);
    assert_eq!(served[0].open_time, now_ms - 180_000);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn api_named_spot_pair_does_not_require_continuous_market_spacing() {
    assert!(!cache_requires_exact_intervals("PURR/USDC", "1m"));
    assert!(cache_requires_exact_intervals("BTC", "1m"));
}

#[test]
fn load_fresh_candles_rejects_tail_older_than_display_window() {
    let root = test_cache_dir("fresh-stale-tail");
    let timeframe = Timeframe::H1;
    let last_time = 1_000_000;
    let last_close_time = last_time + 59_999;
    let now_ms = last_close_time + timeframe.cache_display_max_age_ms() + 1;
    save_candle_snapshot(
        root.clone(),
        ChartBackfillSource::Hyperliquid,
        "BTC",
        timeframe.api_str(),
        vec![Candle::test_flat(last_time, 100.0)],
    )
    .expect("snapshot save succeeds");

    let served = load_fresh_candles_from_dir(
        &root,
        ChartBackfillSource::Hyperliquid,
        "BTC",
        timeframe,
        now_ms,
    )
    .expect("load succeeds");

    assert!(served.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn candle_range_load_misses_across_interior_gap() {
    let root = test_cache_dir("range-gap");
    let now_ms = 10_000_000_000;
    let candles = gapped_hype_snapshot(now_ms);
    let span_start = candles[0].open_time;
    save_candle_snapshot(
        root.clone(),
        ChartBackfillSource::Hyperliquid,
        "HYPE",
        "1m",
        candles,
    )
    .expect("snapshot save succeeds");

    // Endpoints are covered, but the range spans the hole — it must MISS so
    // the caller refetches instead of serving a gapped subset.
    let spanning = load_candles_for_range_from_dir(
        &root,
        ChartBackfillSource::Hyperliquid,
        "HYPE",
        "1m",
        span_start,
        now_ms,
    )
    .expect("range load succeeds");
    assert!(spanning.is_none());

    // A snapshot with any uncertified hole is never allowed to suppress a
    // provider request, even when the requested range happens to sit in its
    // recent tail. Startup display may use that contained tail, but fetch
    // cache hits require whole-snapshot completeness metadata.
    let recent_only = load_candles_for_range_from_dir(
        &root,
        ChartBackfillSource::Hyperliquid,
        "HYPE",
        "1m",
        now_ms - 180_000,
        now_ms,
    )
    .expect("range load succeeds");
    assert!(recent_only.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn gapped_snapshot_is_not_certified_complete_through_last() {
    let root = test_cache_dir("complete-gap");
    let now_ms = 10_000_000_000;
    save_candle_snapshot(
        root.clone(),
        ChartBackfillSource::Hyperliquid,
        "HYPE",
        "1m",
        gapped_hype_snapshot(now_ms),
    )
    .expect("snapshot save succeeds");

    let cached = load_json::<Vec<Candle>>(
        &root,
        CANDLE_CACHE_NAMESPACE,
        &candle_key(ChartBackfillSource::Hyperliquid, "HYPE", "1m"),
    )
    .expect("load succeeds")
    .expect("snapshot exists");
    assert_eq!(cached.complete_through_ms, None);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn snapshot_excludes_candle_that_was_still_open_when_observed() {
    let root = test_cache_dir("closed-only");
    let closed = Candle::test_ohlcv(60_000, 119_999, [100.0, 101.0, 99.0, 100.5], 10.0);
    let forming = Candle::test_ohlcv(120_000, 179_999, [100.5, 110.0, 100.0, 109.0], 20.0);

    save_candle_snapshot_at(
        root.clone(),
        ChartBackfillSource::Hyperliquid,
        "BTC",
        "1m",
        vec![closed, forming],
        150_000,
    )
    .expect("snapshot save succeeds");

    let cached = load_json::<Vec<Candle>>(
        &root,
        CANDLE_CACHE_NAMESPACE,
        &candle_key(ChartBackfillSource::Hyperliquid, "BTC", "1m"),
    )
    .expect("load succeeds")
    .expect("snapshot exists");
    assert_eq!(cached.payload.len(), 1);
    assert_eq!(cached.payload[0].open_time, 60_000);
    assert_eq!(cached.complete_through_ms, Some(119_999));
    assert_eq!(cached.fetched_at_ms, 150_000);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn incomplete_snapshot_cannot_satisfy_a_range_from_endpoint_coverage_alone() {
    let root = test_cache_dir("incomplete-range");
    let candles = vec![
        Candle::test_flat(60_000, 100.0),
        Candle::test_flat(180_000, 102.0),
    ];
    save_candle_snapshot_at(
        root.clone(),
        ChartBackfillSource::Hyperliquid,
        "BTC",
        "1m",
        candles,
        300_000,
    )
    .expect("snapshot save succeeds");

    let cached = load_candles_for_range_from_dir(
        &root,
        ChartBackfillSource::Hyperliquid,
        "BTC",
        "1m",
        60_000,
        240_000,
    )
    .expect("range load succeeds");
    assert!(cached.is_none());

    let _ = fs::remove_dir_all(root);
}
