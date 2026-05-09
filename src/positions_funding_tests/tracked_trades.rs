use crate::app_state::TradingTerminal;
use crate::feed_state::{TrackedTradeFeedRow, TrackedTradeIntent};
use crate::ws::TrackedTradeEvent;
use std::collections::VecDeque;

fn tracked_trade_event() -> TrackedTradeEvent {
    TrackedTradeEvent {
        address: "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string(),
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
        hash: "0xabc".to_string(),
        oid: Some(9),
        tx_index: 3,
    }
}

#[test]
fn events_normalize_wallet_address_before_storage() {
    let mut trade = tracked_trade_event();
    trade.address = "  0xEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE  ".to_string();
    trade.size = 2.0;
    trade.time_ms = 1;

    let normalized = TradingTerminal::normalize_tracked_trade_event(trade);

    assert_eq!(
        normalized.address,
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
    );
}

#[test]
fn dedupe_key_normalizes_address_and_hash_case() {
    let mut trade = tracked_trade_event();
    trade.address = "0xEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE".to_string();
    trade.hash = "0xABC".to_string();
    trade.time_ms = 1;
    trade.size = 2.0;

    let first = TradingTerminal::tracked_trade_event_key(&trade);
    trade.address = trade.address.to_lowercase();
    trade.hash = trade.hash.to_lowercase();
    let second = TradingTerminal::tracked_trade_event_key(&trade);

    assert_eq!(first, second);
}

#[test]
fn intent_uses_start_position_and_signed_delta() {
    assert_eq!(
        TrackedTradeIntent::from_positions(Some(0.0), 2.0),
        TrackedTradeIntent::Opening
    );
    assert_eq!(
        TrackedTradeIntent::from_positions(Some(1.0), 2.0),
        TrackedTradeIntent::Increasing
    );
    assert_eq!(
        TrackedTradeIntent::from_positions(Some(2.0), -1.0),
        TrackedTradeIntent::Reducing
    );
    assert_eq!(
        TrackedTradeIntent::from_positions(Some(2.0), -2.0),
        TrackedTradeIntent::Closing
    );
    assert_eq!(
        TrackedTradeIntent::from_positions(Some(1.0), -2.0),
        TrackedTradeIntent::Reversing
    );
}

#[test]
fn rows_aggregate_order_fills_with_weighted_price() {
    let mut first = tracked_trade_event();
    let mut row = TrackedTradeFeedRow::from_event(&first);
    first.price = 12.0;
    first.size = 2.0;
    first.time_ms = 1_100;
    first.fee = 0.02;
    first.tid = Some(8);

    assert!(row.can_merge(&first));
    row.add_event(&first);

    assert_eq!(row.fill_count, 2);
    assert_eq!(row.size, 3.0);
    assert_eq!(row.notional, 34.0);
    assert!((row.avg_price - (34.0 / 3.0)).abs() < 1e-9);
    assert_eq!(row.fee, 0.03);
    assert_eq!(row.intent, TrackedTradeIntent::Opening);
}

#[test]
fn rows_prefer_hash_before_time_bucket() {
    let mut first = tracked_trade_event();
    first.oid = None;
    let mut row = TrackedTradeFeedRow::from_event(&first);

    let mut different_hash = first.clone();
    different_hash.hash = "0xdef".to_string();
    different_hash.time_ms = 2_000;
    assert!(!row.can_merge(&different_hash));

    let mut same_hash = first.clone();
    same_hash.price = 12.0;
    same_hash.size = 2.0;
    same_hash.time_ms = 2_000;
    same_hash.tid = Some(8);

    assert!(row.can_merge(&same_hash));
    row.add_event(&same_hash);

    assert_eq!(row.fill_count, 2);
    assert_eq!(row.size, 3.0);
    assert!((row.avg_price - (34.0 / 3.0)).abs() < 1e-9);
}

#[test]
fn alerts_emit_each_fill_in_fill_mode() {
    let first = tracked_trade_event();
    let mut existing = VecDeque::new();
    existing.push_front(first.clone());

    let mut second = first;
    second.tid = Some(8);
    second.time_ms = 1_100;

    let alert = TradingTerminal::tracked_trade_alert_row_for_event_from(&existing, false, &second);

    assert!(alert.is_some());
}

#[test]
fn alerts_suppress_merged_order_fills_in_order_mode() {
    let first = tracked_trade_event();
    let mut existing = VecDeque::new();
    existing.push_front(first.clone());

    let mut second = first;
    second.tid = Some(8);
    second.time_ms = 1_100;

    let alert = TradingTerminal::tracked_trade_alert_row_for_event_from(&existing, true, &second);

    assert!(alert.is_none());
}

#[test]
fn alerts_emit_new_orders_in_order_mode() {
    let first = tracked_trade_event();
    let mut existing = VecDeque::new();
    existing.push_front(first.clone());

    let mut second = first;
    second.tid = Some(8);
    second.hash = "0xdef".to_string();
    second.oid = Some(10);
    second.time_ms = 1_100;

    let alert = TradingTerminal::tracked_trade_alert_row_for_event_from(&existing, true, &second);

    assert!(alert.is_some());
}
