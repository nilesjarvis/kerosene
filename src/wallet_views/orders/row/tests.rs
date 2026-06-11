use super::*;

#[test]
fn wallet_order_notional_requires_valid_size_and_price() {
    assert_eq!(wallet_order_notional(Some(2.0), Some(100.0)), Some(200.0));
    assert_eq!(wallet_order_notional(None, Some(100.0)), None);
    assert_eq!(wallet_order_notional(Some(2.0), None), None);
}

#[test]
fn wallet_order_size_marks_invalid_values() {
    assert_eq!(format_wallet_order_size(Some(2.5), false), "2.5000");
    assert_eq!(format_wallet_order_size(None, false), "Invalid data");
    assert_eq!(format_wallet_order_size(None, true), "Invalid data");
}

#[test]
fn wallet_order_size_formats_outcome_orders_as_whole_contracts() {
    assert_eq!(format_wallet_order_size(Some(5.0), true), "5");
    assert_eq!(format_wallet_order_size(Some(12.0), true), "12");
}
