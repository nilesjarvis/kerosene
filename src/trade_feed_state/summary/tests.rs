use super::*;
use crate::feed_state::TrackedTradeIntent;
use crate::trade_feed_state::{TRADE_FEED_CAPACITY, TradeFeedEvent};

fn trade(minute: u64, tid: u64, is_buy: bool, value: f64) -> TradePrint {
    TradePrint {
        time_ms: minute * 60_000 + 1,
        tid,
        is_buy,
        price: value.to_string(),
        size: "1".into(),
        value,
        trader: None,
        intent: TrackedTradeIntent::Unknown,
    }
}

#[test]
fn summary_uses_liquidation_timeframes_and_separates_buys_from_sells() {
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.apply(TradeFeedEvent::Trades(vec![
        trade(59, 1, true, 999.0),
        trade(60, 2, true, 500.0),
        trade(105, 3, false, 400.0),
        trade(115, 4, true, 300.0),
        trade(119, 5, false, 200.0),
        trade(120, 6, true, 100.0),
    ]));
    let now_ms = 120 * 60_000 + 30_000;
    assert_eq!(feed.calculate_summary(1, now_ms), (100.0, 200.0));
    assert_eq!(feed.calculate_summary(5, now_ms), (400.0, 200.0));
    assert_eq!(feed.calculate_summary(15, now_ms), (400.0, 600.0));
    assert_eq!(feed.calculate_summary(60, now_ms), (900.0, 600.0));
}

#[test]
fn summary_survives_row_eviction_and_deduplicates_reconnect_snapshots() {
    let mut feed = TradeFeedInstance::new(Default::default());
    let trades: Vec<_> = (0..1_000).map(|tid| trade(120, tid, true, 2.0)).collect();
    feed.apply(TradeFeedEvent::Trades(trades.clone()));
    assert_eq!(feed.trades.len(), TRADE_FEED_CAPACITY);
    assert_eq!(feed.calculate_summary(1, 120 * 60_000), (2_000.0, 0.0));
    feed.apply(TradeFeedEvent::Reconnecting);
    feed.apply(TradeFeedEvent::Trades(trades));
    assert_eq!(feed.calculate_summary(1, 120 * 60_000), (2_000.0, 0.0));
    assert!(feed.has_gap);
}

#[test]
fn summary_stays_live_while_rows_are_held_grouped_and_filtered() {
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.apply(TradeFeedEvent::Trades(vec![trade(120, 1, true, 100.0)]));
    let hover_key = feed.address_hover_key(7, &feed.trades[0]);
    feed.sync_address_hover(7, Some(&hover_key));
    feed.group_trades = true;
    feed.min_group_notional = 1_000.0;
    feed.size_in_notional = true;
    feed.apply(TradeFeedEvent::Trades(vec![trade(120, 2, false, 200.0)]));
    assert!(feed.is_held());
    assert!(feed.visible_rows().is_empty());
    assert_eq!(feed.calculate_summary(1, 120 * 60_000), (100.0, 200.0));
}

#[test]
fn summary_expires_without_new_trades_and_handles_clock_skew() {
    let mut feed = TradeFeedInstance::new(Default::default());
    assert_eq!(feed.calculate_summary(60, 0), (0.0, 0.0));
    feed.apply(TradeFeedEvent::Trades(vec![trade(120, 1, true, 100.0)]));
    assert_eq!(feed.calculate_summary(1, 119 * 60_000), (0.0, 0.0));
    assert_eq!(feed.calculate_summary(1, 122 * 60_000), (0.0, 0.0));
    assert_eq!(feed.calculate_summary(60, 180 * 60_000), (100.0, 0.0));
    assert_eq!(feed.calculate_summary(60, 181 * 60_000), (0.0, 0.0));
}

#[test]
fn summary_prunes_old_buckets_and_accepts_out_of_order_prints() {
    let mut feed = TradeFeedInstance::new(Default::default());
    for minute in 1..=120 {
        feed.apply(TradeFeedEvent::Trades(vec![trade(minute, 1, true, 1.0)]));
    }
    assert_eq!(feed.summary_buckets.len(), 61);
    assert_eq!(feed.calculate_summary(60, 120 * 60_000), (61.0, 0.0));
    feed.apply(TradeFeedEvent::Trades(vec![
        trade(59, 2, false, 999.0),
        trade(115, 2, false, 100.0),
    ]));
    assert_eq!(feed.summary_buckets.len(), 61);
    assert_eq!(feed.calculate_summary(5, 120 * 60_000), (6.0, 100.0));
    assert_eq!(feed.calculate_summary(60, 120 * 60_000), (61.0, 100.0));
}

#[test]
fn summary_is_per_instance_and_cleared_with_stream_reset() {
    let mut feed = TradeFeedInstance::new(Default::default());
    let other_feed = TradeFeedInstance::new(Default::default());
    feed.apply(TradeFeedEvent::Trades(vec![trade(120, 1, true, 100.0)]));
    assert_eq!(other_feed.calculate_summary(60, 120 * 60_000), (0.0, 0.0));
    feed.reset(Some("ETH".into()), 2);
    assert_eq!(feed.calculate_summary(60, 120 * 60_000), (0.0, 0.0));
    assert!(feed.summary_buckets.is_empty());
    feed.apply(TradeFeedEvent::Trades(vec![trade(120, 1, false, 200.0)]));
    assert_eq!(feed.calculate_summary(60, 120 * 60_000), (0.0, 200.0));
}
