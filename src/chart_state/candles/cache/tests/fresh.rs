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
    let now_ms = Timeframe::M1.lookback_ms();

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
fn get_fresh_cached_candles_evicts_stale_entries() {
    let mut cache = HashMap::from([(cache_key("BTC", Timeframe::M1), vec![candle(0, 100.0)])]);
    let mut order = VecDeque::from([cache_key("BTC", Timeframe::M1)]);
    let now_ms = Timeframe::M1.lookback_ms() + 1;

    let candles = get_fresh_cached_candles(&mut cache, &mut order, "BTC", Timeframe::M1, now_ms);

    assert!(candles.is_none());
    assert!(cache.is_empty());
    assert!(order.is_empty());
}
