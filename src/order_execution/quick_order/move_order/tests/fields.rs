use super::super::{move_order_wire_is_supported, moved_order_is_buy};
use super::fixtures::open_order;

#[test]
fn moved_order_side_accepts_only_exchange_bid_or_ask_markers() {
    assert_eq!(moved_order_is_buy("B"), Some(true));
    assert_eq!(moved_order_is_buy("A"), Some(false));
    assert_eq!(moved_order_is_buy("buy"), None);
    assert_eq!(moved_order_is_buy(""), None);
}

#[test]
fn move_order_wire_rejects_trigger_orders() {
    let mut order = open_order("BTC", 42, "100");
    order.is_trigger = Some(true);
    order.trigger_px = Some("95".to_string());

    let error = move_order_wire_is_supported(&order).unwrap_err();

    assert!(error.contains("trigger orders"));
}

#[test]
fn move_order_wire_accepts_limit_orders_with_zero_trigger_px_metadata() {
    let mut order = open_order("BTC", 42, "100");
    order.is_trigger = Some(false);
    order.trigger_px = Some("0.0".to_string());

    assert!(move_order_wire_is_supported(&order).is_ok());
}

#[test]
fn move_order_wire_rejects_known_non_gtc_orders() {
    let mut order = open_order("BTC", 42, "100");
    order.tif = Some("Ioc".to_string());

    let error = move_order_wire_is_supported(&order).unwrap_err();

    assert!(error.contains("non-GTC"));
}
