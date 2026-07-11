use crate::ws::TrackedTradeEvent;

use super::*;

fn event() -> TrackedTradeEvent {
    TrackedTradeEvent {
        address: "0xabc".to_string(),
        coin: "HYPE".to_string(),
        price: 10.0,
        size: 1.0,
        is_buy: true,
        time_ms: 1_000,
        dir: "Open Long".to_string(),
        start_position: Some(0.0),
        closed_pnl: 0.0,
        fee: 0.01,
        fee_token: "USDC".to_string(),
        tid: Some(7),
        hash: "0xhash".to_string(),
        oid: Some(9),
        tx_index: 3,
    }
}

#[test]
fn row_can_merge_same_order_id_across_wide_time_gap() {
    let row = TrackedTradeFeedRow::from_event(&event());
    let mut later = event();
    later.time_ms += TRACKED_TRADE_AGGREGATION_WINDOW_MS + 10_000;

    assert!(row.can_merge(&later));
}

#[test]
fn row_rejects_different_hash_outside_time_window() {
    let mut first = event();
    first.oid = None;
    let row = TrackedTradeFeedRow::from_event(&first);

    let mut later = first.clone();
    later.hash = "0xother".to_string();
    later.time_ms += TRACKED_TRADE_AGGREGATION_WINDOW_MS + 1;

    assert!(!row.can_merge(&later));
}

#[test]
fn row_add_event_tracks_mixed_fee_token_direction_and_earlier_start() {
    let mut row = TrackedTradeFeedRow::from_event(&event());
    let mut earlier = event();
    earlier.time_ms = 900;
    earlier.price = 12.0;
    earlier.size = 2.0;
    earlier.dir = "Close Long".to_string();
    earlier.start_position = Some(-3.0);
    earlier.fee_token = "HYPE".to_string();

    row.add_event(&earlier);

    assert_eq!(row.first_time_ms, 900);
    assert_eq!(row.last_time_ms, 1_000);
    assert_eq!(row.fill_count, 2);
    assert_eq!(row.size, 3.0);
    assert_eq!(row.notional, 34.0);
    assert!((row.avg_price - (34.0 / 3.0)).abs() < 1e-9);
    assert_eq!(row.fee_token, "mixed");
    assert_eq!(row.dir, "Mixed");
    assert_eq!(row.start_position, Some(-3.0));
}

#[test]
fn tracked_trade_row_debug_redacts_account_trade_values_without_changing_them() {
    let mut trade = event();
    trade.address = "private-row-address-sentinel".to_string();
    trade.coin = "private-row-coin-sentinel".to_string();
    trade.price = 98_765.432_1;
    trade.size = 12_345.678_9;
    trade.hash = "private-row-hash-sentinel".to_string();
    trade.oid = Some(98_765_432);
    let row = TrackedTradeFeedRow::from_event(&trade);

    let rendered = format!("{row:?}");

    assert!(rendered.contains("<redacted>"), "{rendered}");
    for sensitive in [
        "private-row-address-sentinel",
        "private-row-coin-sentinel",
        "private-row-hash-sentinel",
        "98765.4321",
        "12345.6789",
        "98765432",
    ] {
        assert!(!rendered.contains(sensitive), "{rendered}");
    }
    assert_eq!(row.address, "private-row-address-sentinel");
    assert_eq!(row.coin, "private-row-coin-sentinel");
    assert_eq!(row.avg_price.to_bits(), 98_765.432_1_f64.to_bits());
    assert_eq!(row.size.to_bits(), 12_345.678_9_f64.to_bits());
    assert_eq!(row.oid, Some(98_765_432));
}
