use super::*;
use crate::trade_feed_state::TradeFeedEvent;

const FIRST: &str = "0x1111111111111111111111111111111111111111";
const SECOND: &str = "0x2222222222222222222222222222222222222222";

fn trade(
    time_ms: u64,
    tid: u64,
    trader: Option<&str>,
    is_buy: bool,
    price: &str,
    size: &str,
) -> TradePrint {
    TradePrint {
        time_ms,
        tid,
        intent: crate::feed_state::TrackedTradeIntent::Unknown,
        trader: trader.map(Into::into),
        is_buy,
        price: price.into(),
        size: size.into(),
        value: price.parse::<f64>().expect("fixture price")
            * size.parse::<f64>().expect("fixture size"),
    }
}

fn feed(trades: Vec<TradePrint>) -> TradeFeedInstance {
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.group_trades = true;
    feed.apply(TradeFeedEvent::Trades(trades));
    feed
}

#[test]
fn trade_feed_sizes_compact_large_quantities_and_preserve_small_prints() {
    for (size, expected) in [
        ("0.00000001", "0.00000001"),
        ("0.100", "0.100"),
        ("999.99", "999.99"),
        ("1000", "1.0K"),
        ("12500", "12.5K"),
        ("1250000", "1.25M"),
        ("2500000000", "2.50B"),
    ] {
        let print = trade(1000, 1, Some(FIRST), true, "1", size);
        assert_eq!(TradeFeedRow::new(&print).size_label(), expected);
        assert_eq!(print.size, size);
    }
    let mut feed = feed(vec![
        trade(1000, 1, Some(FIRST), true, "1", "600.25"),
        trade(999, 2, Some(FIRST), true, "1", "649.75"),
    ]);
    let rows = feed.visible_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].size_label(), "1.2K");
    assert_eq!(rows[0].value, 1250.0);
    feed.group_trades = false;
    assert_eq!(feed.visible_rows()[0].size_label(), "600.25");
    assert_eq!(feed.visible_rows()[1].size_label(), "649.75");
}

#[test]
fn trade_feed_grouping_combines_interleaved_same_side_trades_and_weights_price_by_size() {
    let feed = feed(vec![
        trade(10_000, 1, Some(FIRST), true, "100", "1"),
        trade(9_750, 2, Some(SECOND), true, "100", "2"),
        trade(9_600, 3, Some(FIRST), false, "100", "3"),
        trade(9_500, 4, Some(FIRST), true, "200", "2"),
        trade(9_400, 5, None, true, "100", "1"),
        trade(9_300, 6, None, true, "100", "1"),
        trade(9_000, 7, Some(FIRST), true, "400", "1"),
        trade(8_999, 8, Some(FIRST), true, "100", "1"),
    ]);
    let rows = feed.visible_rows();
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[0].count, 3);
    assert_eq!(rows[0].size_label(), "4");
    assert_eq!(rows[0].value, 900.0);
    assert_eq!(rows[0].price_label(), "225");
    assert_eq!(
        rows.iter().map(|row| row.latest.tid).collect::<Vec<_>>(),
        vec![1, 2, 3, 5, 6, 8]
    );
    assert!(rows[1..].iter().all(|row| row.count == 1));
}

#[test]
fn trade_feed_grouping_does_not_chain_bursts_or_consume_raw_prints() {
    let mut feed = feed(vec![
        trade(10_000, 1, Some(FIRST), true, "100.00", "0.100"),
        trade(9_400, 2, Some(FIRST), true, "100.00", "0.200"),
        trade(8_800, 3, Some(FIRST), true, "100.00", "0.300"),
    ]);
    let rows = feed.visible_rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].count, 2);
    assert_eq!(rows[0].size_label(), "0.3");
    feed.group_trades = false;
    let rows = feed.visible_rows();
    assert_eq!(rows.len(), 3);
    assert!(
        rows.iter()
            .all(|row| row.count == 1 && row.price_label() == "100.00")
    );
    assert_eq!(rows[0].size_label(), "0.100");
    feed.group_trades = true;
    assert_eq!(feed.visible_rows()[0].count, 2);
    assert_eq!(feed.trades.len(), 3);
}

#[test]
fn trade_feed_grouping_keeps_small_numbers_and_refuses_overflow() {
    let feed = feed(vec![
        trade(1000, 1, Some(FIRST), true, "0.00000001", "0.00001"),
        trade(999, 2, Some(FIRST), true, "0.00000002", "0.00001"),
        trade(998, 3, Some(SECOND), true, "1e308", "1"),
        trade(997, 4, Some(SECOND), true, "1e308", "1"),
    ]);
    let rows = feed.visible_rows();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].price_label(), "0.000000015");
    assert_eq!(rows[0].size_label(), "0.00002");
    assert!(rows.iter().all(|row| row.value.is_finite()));
}

#[test]
fn trade_feed_grouping_uses_deduplicated_prints_and_holds_aggregates_during_hover() {
    let original = trade(1000, 1, Some(FIRST), true, "10", "1");
    let mut feed = feed(vec![original.clone(), original]);
    feed.apply(TradeFeedEvent::Trades(vec![trade(
        1001,
        2,
        Some(FIRST),
        true,
        "20",
        "1",
    )]));
    assert_eq!(feed.visible_rows()[0].count, 2);
    let hover_key = feed.address_hover_key(7, feed.visible_rows()[0].latest);
    feed.sync_address_hover(7, Some(&hover_key));
    feed.apply(TradeFeedEvent::Trades(vec![trade(
        1002,
        3,
        Some(FIRST),
        true,
        "30",
        "1",
    )]));
    assert_eq!(feed.visible_rows()[0].count, 2);
    assert_eq!(feed.visible_rows()[0].value, 30.0);
    feed.sync_address_hover(7, None);
    assert_eq!(feed.visible_rows()[0].count, 3);
    assert_eq!(feed.visible_rows()[0].value, 60.0);
    feed.reset(Some("ETH".into()), 1);
    assert!(feed.visible_rows().is_empty());
    assert!(feed.group_trades);
}

#[test]
fn trade_feed_grouping_keeps_different_position_actions_separate() {
    use crate::feed_state::TrackedTradeIntent;
    let mut open = trade(1000, 1, Some(FIRST), true, "10", "1");
    open.intent = TrackedTradeIntent::Opening;
    let mut add = trade(1100, 2, Some(FIRST), true, "20", "1");
    add.intent = TrackedTradeIntent::Increasing;
    let mut more = trade(1200, 3, Some(FIRST), true, "30", "1");
    more.intent = TrackedTradeIntent::Increasing;
    let feed = feed(vec![open, add, more]);
    let rows = feed.visible_rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].latest.intent, TrackedTradeIntent::Increasing);
    assert_eq!(rows[0].count, 2);
    assert_eq!(rows[0].value, 50.0);
    assert_eq!(rows[1].latest.intent, TrackedTradeIntent::Opening);
    assert_eq!(rows[1].count, 1);
}
