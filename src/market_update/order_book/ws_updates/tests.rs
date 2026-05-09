use crate::api::{BookLevel, OrderBook};
use crate::market_state::OrderBookSymbolMode;
use crate::signing::ChaseOrder;
use std::time::Instant;

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
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        remaining_size: 1.0,
        asset: 0,
        sz_decimals: 3,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 98.0,
        initial_price: 98.0,
        started_at,
        reprice_count: 0,
        cancel_in_flight: false,
        stop_requested: false,
        cancel_retries: 0,
        oid_confirmed: true,
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

    assert!(chase_should_reprice(&chase, "BTC", "BTC", Some(99.0)));
    assert!(!chase_should_reprice(&chase, "ETH", "BTC", Some(99.0)));
    assert!(!chase_should_reprice(&chase, "BTC", "ETH", Some(99.0)));
    assert!(!chase_should_reprice(&chase, "BTC", "BTC", None));
    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(98.0)));
}

#[test]
fn chase_reprice_waits_while_cancel_is_in_flight() {
    let mut chase = chase();
    chase.cancel_in_flight = true;

    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(99.0)));
}

#[test]
fn chase_reprice_waits_after_stop_is_requested() {
    let mut chase = chase();
    chase.stop_requested = true;

    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(99.0)));
}
