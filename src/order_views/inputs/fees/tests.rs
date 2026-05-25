use super::order_fee_quantity;

#[test]
fn usd_fee_quantity_converts_notional_to_base_size() {
    assert_eq!(order_fee_quantity("250", 100.0, true, 5), Some(2.5));
}

#[test]
fn coin_fee_quantity_keeps_base_size() {
    assert_eq!(order_fee_quantity("2.5", 100.0, false, 5), Some(2.5));
}

#[test]
fn usd_fee_quantity_uses_asset_precision() {
    assert_eq!(order_fee_quantity("10", 30_000.0, true, 5), Some(0.00033));
    assert_eq!(order_fee_quantity("10", 30_000.0, true, 2), None);
}
