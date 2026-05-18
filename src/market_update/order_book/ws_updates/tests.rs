use crate::api::{BookLevel, OrderBook};
use crate::market_state::OrderBookSymbolMode;
use crate::signing::{ChaseOrder, ChasePendingOp};
use std::time::{Duration, Instant};

use super::*;

fn book() -> OrderBook {
    OrderBook {
        bids: vec![BookLevel { px: 99.0, sz: 1.0 }],
        asks: vec![BookLevel { px: 101.0, sz: 1.0 }],
    }
}

fn chase() -> ChaseOrder {
    let started_at = Instant::now();
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 98.0,
        current_price_wire: "98".to_string(),
        initial_price: 98.0,
        started_at,
        started_at_ms: 1_000,
        adopted_resting: false,
        reprice_count: 0,
        pending_op: None,
        last_reprice_at: None,
        pending_best_price: None,
        pending_size_correction: false,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: true,
        missing_open_order_refresh_requested: false,
    }
}

#[test]
fn order_book_track_check_matches_active_or_fixed_symbol() {
    assert!(order_book_tracks_coin(
        &OrderBookSymbolMode::Active,
        "BTC",
        "BTC"
    ));
    assert!(!order_book_tracks_coin(
        &OrderBookSymbolMode::Active,
        "ETH",
        "BTC"
    ));
    assert!(order_book_tracks_coin(
        &OrderBookSymbolMode::Fixed("BTC".to_string()),
        "ETH",
        "BTC"
    ));
    assert!(!order_book_tracks_coin(
        &OrderBookSymbolMode::Fixed("ETH".to_string()),
        "BTC",
        "BTC"
    ));
}

#[test]
fn best_chase_price_uses_bid_for_buy_and_ask_for_sell() {
    let book = book();

    assert_eq!(best_chase_price(&book, true), Some(99.0));
    assert_eq!(best_chase_price(&book, false), Some(101.0));
    assert_eq!(best_chase_price(&OrderBook::empty(), true), None);
    assert_eq!(best_chase_price(&OrderBook::empty(), false), None);
}

#[test]
fn best_chase_price_rejects_invalid_book_levels() {
    let invalid_book = OrderBook {
        bids: vec![BookLevel {
            px: f64::INFINITY,
            sz: 1.0,
        }],
        asks: vec![BookLevel { px: 0.0, sz: 1.0 }],
    };

    assert_eq!(best_chase_price(&invalid_book, true), None);
    assert_eq!(best_chase_price(&invalid_book, false), None);
}

#[test]
fn chase_reprices_only_when_symbol_active_ready_and_price_changed() {
    let chase = chase();

    let now = Instant::now();

    assert!(chase_should_reprice(&chase, "BTC", "BTC", Some(99.0), now));
    assert!(!chase_should_reprice(&chase, "ETH", "BTC", Some(99.0), now));
    assert!(!chase_should_reprice(&chase, "BTC", "ETH", Some(99.0), now));
    assert!(!chase_should_reprice(&chase, "BTC", "BTC", None, now));
    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(98.0), now));
    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(97.0), now));
}

#[test]
fn sell_chase_reprices_only_toward_lower_prices() {
    let mut chase = chase();
    chase.is_buy = false;
    chase.current_price = 172.0;
    chase.current_price_wire = "172".to_string();

    let now = Instant::now();

    assert!(chase_should_reprice(&chase, "BTC", "BTC", Some(171.8), now));
    assert!(!chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(172.2),
        now
    ));
}

#[test]
fn chase_reprice_waits_while_operation_is_in_flight() {
    let mut chase = chase();
    chase.pending_op = Some(ChasePendingOp::Modify { oid: 42 });

    assert!(!chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(99.0),
        Instant::now()
    ));
}

#[test]
fn chase_reprice_waits_after_stop_is_requested() {
    let mut chase = chase();
    chase.stop_requested = true;

    assert!(!chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(99.0),
        Instant::now()
    ));
}

#[test]
fn chase_reprice_compares_rounded_wire_price() {
    let mut chase = chase();
    chase.sz_decimals = 2;
    chase.current_price = 100.0;
    chase.current_price_wire = "100".to_string();

    assert!(!chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(100.001),
        Instant::now()
    ));
    assert!(chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(101.0),
        Instant::now()
    ));
}

#[test]
fn chase_reprice_waits_for_minimum_interval() {
    let mut chase = chase();
    let now = Instant::now();
    let min_interval = Duration::from_secs(1);
    chase.last_reprice_at = Some(now - min_interval + Duration::from_millis(1));

    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(99.0), now));

    chase.last_reprice_at = Some(now - min_interval);
    assert!(chase_should_reprice(&chase, "BTC", "BTC", Some(99.0), now));
}
