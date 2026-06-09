use super::*;

#[test]
fn store_normalized_candles_ignores_empty_results() {
    let mut cache = HashMap::new();
    let mut order = VecDeque::new();

    store_normalized_candles(&mut cache, &mut order, "BTC", Timeframe::M1, Vec::new());

    assert!(cache.is_empty());
    assert!(order.is_empty());
}

#[test]
fn store_normalized_candles_sorts_and_moves_key_to_back() {
    let mut cache = HashMap::new();
    let mut order = VecDeque::from([cache_key("ETH", Timeframe::M1)]);

    store_normalized_candles(
        &mut cache,
        &mut order,
        "ETH",
        Timeframe::M1,
        vec![candle(2_000, 102.0), candle(1_000, 101.0)],
    );

    let cached = cached_candles_or_panic(&cache, "ETH", Timeframe::M1);
    assert_eq!(cached[0].open_time, 1_000);
    assert_eq!(cached[1].open_time, 2_000);
    assert_eq!(order, VecDeque::from([cache_key("ETH", Timeframe::M1)]));
}

#[test]
fn store_cached_candles_keeps_cache_ready_data_and_moves_key_to_back() {
    let mut cache = HashMap::new();
    let mut order = VecDeque::from([cache_key("BTC", Timeframe::M1)]);

    store_cached_candles(
        &mut cache,
        &mut order,
        "BTC",
        Timeframe::M1,
        vec![candle(1_000, 101.0), candle(2_000, 102.0)],
    );

    let cached = cached_candles_or_panic(&cache, "BTC", Timeframe::M1);
    assert_eq!(cached[0].open_time, 1_000);
    assert_eq!(cached[1].open_time, 2_000);
    assert_eq!(order, VecDeque::from([cache_key("BTC", Timeframe::M1)]));
}

#[test]
fn store_normalized_candles_evicts_oldest_key_after_capacity() {
    let mut cache = HashMap::new();
    let mut order = VecDeque::new();

    for idx in 0..=CANDLE_CACHE_CAPACITY {
        let symbol = format!("COIN{idx}");
        store_normalized_candles(
            &mut cache,
            &mut order,
            &symbol,
            Timeframe::M1,
            vec![candle(idx as u64 * 60_000, idx as f64)],
        );
    }

    assert_eq!(cache.len(), CANDLE_CACHE_CAPACITY);
    assert_eq!(order.len(), CANDLE_CACHE_CAPACITY);
    assert!(!cache.contains_key(&cache_key("COIN0", Timeframe::M1)));
    assert_eq!(order.front(), Some(&cache_key("COIN1", Timeframe::M1)));
}
