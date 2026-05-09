use super::*;

#[test]
fn wallet_order_notional_requires_valid_size_and_price() {
    assert_eq!(wallet_order_notional(Some(2.0), Some(100.0)), Some(200.0));
    assert_eq!(wallet_order_notional(None, Some(100.0)), None);
    assert_eq!(wallet_order_notional(Some(2.0), None), None);
}

#[test]
fn wallet_order_size_marks_invalid_values() {
    assert_eq!(format_wallet_order_size(Some(2.5)), "2.5000");
    assert_eq!(format_wallet_order_size(None), "Invalid data");
}
