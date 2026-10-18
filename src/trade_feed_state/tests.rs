use super::*;

fn trade(time_ms: u64, tid: u64) -> TradePrint {
    TradePrint {
        time_ms,
        tid,
        is_buy: true,
        price: "100".into(),
        size: "2".into(),
        value: 200.0,
    }
}

#[test]
fn overlap_is_deduplicated_and_older_snapshots_do_not_replace_newer_trades() {
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.apply(TradeFeedEvent::Trades(vec![trade(20, 7), trade(20, 8)]));
    feed.apply(TradeFeedEvent::Trades(vec![trade(10, 7), trade(20, 7)]));
    assert_eq!(
        feed.trades
            .iter()
            .map(|t| (t.time_ms, t.tid))
            .collect::<Vec<_>>(),
        vec![(20, 8), (20, 7), (10, 7)]
    );
}

#[test]
fn burst_memory_is_bounded_and_reset_forgets_old_asset() {
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.apply(TradeFeedEvent::Trades(
        (1..6000).map(|i| trade(i, i)).collect(),
    ));
    assert_eq!(feed.trades.len(), TRADE_FEED_CAPACITY);
    assert_eq!(feed.seen.len(), SEEN_CAPACITY);
    assert_eq!(feed.trades.front().map(|t| t.tid), Some(5999));
    feed.apply(TradeFeedEvent::Lagged);
    feed.reset(Some("ETH".into()), 4);
    assert!(feed.trades.is_empty() && feed.seen.is_empty() && !feed.has_gap);
    assert_eq!(feed.generation, 4);
}

#[test]
fn reconnect_retains_rows_and_does_not_claim_complete_history_after_new_prints() {
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.apply(TradeFeedEvent::Trades(vec![trade(20, 7)]));
    feed.apply(TradeFeedEvent::Reconnecting);
    feed.apply(TradeFeedEvent::Trades(vec![trade(21, 8)]));
    assert!(feed.has_gap);
    assert_eq!(feed.status, "Live");
    assert_eq!(feed.trades.len(), 2);
}

#[test]
fn shading_is_monotonic_bounded_and_independent_of_asset_units() {
    assert!(trade_shading(10.0, 100.0) < trade_shading(50.0, 100.0));
    assert_eq!(trade_shading(10.0, 100.0), trade_shading(1000.0, 10000.0));
    assert!(trade_shading(1000.0, 100.0) <= 0.31);
    assert_eq!(trade_shading(f64::NAN, 100.0), 0.0);
    assert_eq!(trade_shading(1.0, 0.0), 0.0);
}

#[test]
fn equal_timestamp_prints_follow_receipt_order_not_the_trade_id_hash() {
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.apply(TradeFeedEvent::Trades(vec![trade(20, 900), trade(20, 3)]));
    assert_eq!(
        feed.trades
            .iter()
            .map(|trade| trade.tid)
            .collect::<Vec<_>>(),
        vec![3, 900]
    );
}

#[test]
fn older_replay_cannot_expire_deduplication_for_still_visible_rows() {
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.apply(TradeFeedEvent::Trades(vec![trade(10_000, 0)]));
    feed.apply(TradeFeedEvent::Trades(
        (1..6000).map(|i| trade(i, i)).collect(),
    ));
    feed.apply(TradeFeedEvent::Trades(vec![trade(10_000, 0)]));
    assert_eq!(feed.trades.iter().filter(|trade| trade.tid == 0).count(), 1);
    assert_eq!(feed.trades.len(), TRADE_FEED_CAPACITY);
}

#[test]
fn trade_feed_picker_prefers_exact_tickers_over_outcome_title_matches() {
    assert_eq!(trade_feed_symbol_rank(" SOL ", "SOL", "SOL"), Some(0));
    assert_eq!(
        trade_feed_symbol_rank("sol", "#123", "YES: SOL is above 100"),
        Some(3)
    );
    assert_eq!(
        trade_feed_symbol_rank("sol", "#124", "Outcome: Solary wins"),
        Some(3)
    );
    assert_eq!(trade_feed_symbol_rank("xyz:btc", "xyz:BTC", "BTC"), Some(0));
    assert_eq!(trade_feed_symbol_rank("eth", "SOL", "SOL"), None);
}
