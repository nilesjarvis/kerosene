use super::*;

#[test]
fn get_fresh_cached_candles_refreshes_lru_order() {
    let mut cache = HashMap::from([
        (cache_key("BTC", Timeframe::M1), vec![candle(1_000, 100.0)]),
        (cache_key("ETH", Timeframe::M1), vec![candle(2_000, 200.0)]),
    ]);
    let mut order = VecDeque::from([
        cache_key("BTC", Timeframe::M1),
        cache_key("ETH", Timeframe::M1),
    ]);
    let now_ms = 1_000 + Timeframe::M1.cache_display_max_age_ms();

    let candles = fresh_candles_or_panic(&mut cache, &mut order, "BTC", Timeframe::M1, now_ms);

    assert_eq!(candles[0].open_time, 1_000);
    assert_eq!(
        order,
        VecDeque::from([
            cache_key("ETH", Timeframe::M1),
            cache_key("BTC", Timeframe::M1)
        ])
    );
}

#[test]
fn get_fresh_cached_candles_returns_only_trailing_run_across_gap() {
    let now_ms = 10_000_000_000;
    let old_start = now_ms - 4 * 24 * 60 * 60 * 1_000;
    let recent_start = now_ms - 180_000;
    let gapped = vec![
        candle(old_start, 60.0),
        candle(old_start + 60_000, 60.0),
        candle(recent_start, 70.0),
        candle(recent_start + 60_000, 70.0),
    ];
    let mut cache = HashMap::from([(cache_key("HYPE", Timeframe::M1), gapped)]);
    let mut order = VecDeque::from([cache_key("HYPE", Timeframe::M1)]);

    let candles = fresh_candles_or_panic(&mut cache, &mut order, "HYPE", Timeframe::M1, now_ms);

    // Recent tail is fresh, but the stale $60 head sits behind a multi-day hole;
    // only the contiguous trailing run is handed back.
    assert_eq!(candles.len(), 2);
    assert_eq!(candles[0].open_time, recent_start);
    assert!(candles.iter().all(|candle| candle.close == 70.0));
}

#[test]
fn get_fresh_cached_candles_evicts_stale_entries() {
    let mut cache = HashMap::from([(cache_key("BTC", Timeframe::M1), vec![candle(0, 100.0)])]);
    let mut order = VecDeque::from([cache_key("BTC", Timeframe::M1)]);
    let now_ms = Timeframe::M1.cache_display_max_age_ms() + 1;

    let candles = get_fresh_cached_candles(&mut cache, &mut order, "BTC", Timeframe::M1, now_ms);

    assert!(candles.is_none());
    assert!(cache.is_empty());
    assert!(order.is_empty());
}
