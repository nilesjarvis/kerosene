use super::UserOrderBookLevels;
use crate::account::OpenOrder;
use crate::app_state::TradingTerminal;
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};

fn open_order(coin: &str, side: &str, limit_px: &str, oid: u64) -> OpenOrder {
    OpenOrder {
        coin: coin.to_string(),
        side: side.to_string(),
        limit_px: limit_px.to_string(),
        sz: "1".to_string(),
        oid,
        timestamp: oid,
        reduce_only: None,
        is_trigger: None,
        order_type: None,
        tif: None,
        trigger_px: None,
    }
}

#[test]
fn user_order_levels_filter_to_symbol_and_side() {
    let orders = vec![
        open_order("BTC", "B", "99.74", 1),
        open_order("BTC", "A", "100.01", 2),
        open_order("ETH", "B", "99.5", 3),
        open_order("BTC", "X", "99.5", 4),
    ];

    let levels = UserOrderBookLevels::from_orders(&orders, "BTC", 0.5);

    assert!(levels.has_bid_at_price(99.5, 0.5));
    assert!(levels.has_ask_at_price(100.5, 0.5));
    assert!(!levels.has_bid_at_price(99.0, 0.5));
    assert!(!levels.has_ask_at_price(100.0, 0.5));
}

#[test]
fn user_order_levels_collapse_multiple_orders_in_same_denomination() {
    let orders = vec![
        open_order("BTC", "B", "99.74", 1),
        open_order("BTC", "B", "99.51", 2),
        open_order("BTC", "A", "100.01", 3),
        open_order("BTC", "A", "100.49", 4),
    ];

    let levels = UserOrderBookLevels::from_orders(&orders, "BTC", 0.5);

    assert_eq!(levels.bids.len(), 1);
    assert_eq!(levels.asks.len(), 1);
    assert!(levels.has_bid_at_price(99.5, 0.5));
    assert!(levels.has_ask_at_price(100.5, 0.5));
}

#[test]
fn user_order_levels_ignore_invalid_inputs() {
    let orders = vec![
        open_order("BTC", "B", "bad", 1),
        open_order("BTC", "A", "NaN", 2),
        open_order("BTC", "B", "0", 3),
    ];

    let invalid_tick = UserOrderBookLevels::from_orders(&orders, "BTC", 0.0);
    let valid_tick = UserOrderBookLevels::from_orders(&orders, "BTC", 0.5);

    assert!(invalid_tick.bids.is_empty());
    assert!(invalid_tick.asks.is_empty());
    assert!(valid_tick.bids.is_empty());
    assert!(valid_tick.asks.is_empty());
}

#[test]
fn resolved_tick_keeps_a_tick_that_is_in_the_option_set() {
    let mut inst = OrderBookInstance::new(1, OrderBookSymbolMode::Active, 0.5);
    inst.set_tick_size(0.1);
    let options = [0.01, 0.05, 0.1, 0.5, 1.0];

    assert_eq!(
        TradingTerminal::resolved_order_book_tick(&inst, &options),
        0.1
    );
}

#[test]
fn resolved_tick_snaps_to_nearest_option_after_a_regime_change() {
    // A persisted tick from a different price regime must land on a real
    // selector button, not silently reset to the default.
    let inst = OrderBookInstance::new(1, OrderBookSymbolMode::Active, 50.0);
    let options = [0.01, 0.05, 0.1, 0.5, 1.0];

    assert_eq!(
        TradingTerminal::resolved_order_book_tick(&inst, &options),
        1.0
    );
}

#[test]
fn user_order_levels_ignore_invalid_display_prices() {
    let orders = vec![open_order("BTC", "B", "99.74", 1)];
    let levels = UserOrderBookLevels::from_orders(&orders, "BTC", 0.5);

    assert!(!levels.has_bid_at_price(0.0, 0.5));
    assert!(!levels.has_bid_at_price(-99.5, 0.5));
    assert!(!levels.has_bid_at_price(f64::NAN, 0.5));
    assert!(!levels.has_bid_at_price(f64::INFINITY, 0.5));
    assert!(!levels.has_bid_at_price(99.5, f64::NAN));
}
