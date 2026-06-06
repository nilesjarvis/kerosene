use super::super::moved_order_is_buy;

#[test]
fn moved_order_side_accepts_only_exchange_bid_or_ask_markers() {
    assert_eq!(moved_order_is_buy("B"), Some(true));
    assert_eq!(moved_order_is_buy("A"), Some(false));
    assert_eq!(moved_order_is_buy("buy"), None);
    assert_eq!(moved_order_is_buy(""), None);
}
