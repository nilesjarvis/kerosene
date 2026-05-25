use super::*;

#[test]
fn order_size_keeps_coin_amount_at_valid_precision() {
    assert_eq!(
        order_size_from_quantity_input(2.5, 100.0, false, 5),
        Some(2.5)
    );
}

#[test]
fn order_size_quantizes_coin_amount_to_asset_precision() {
    assert_eq!(
        order_size_from_quantity_input(1.23, 100.0, false, 2),
        Some(1.23)
    );
    assert_eq!(
        order_size_from_quantity_input(1.239, 100.0, false, 2),
        Some(1.23)
    );
}

#[test]
fn order_size_converts_usd_amount_by_price() {
    assert_eq!(
        order_size_from_quantity_input(250.0, 100.0, true, 5),
        Some(2.5)
    );
}

#[test]
fn order_size_quantizes_usd_conversion_to_asset_precision() {
    assert_eq!(
        order_size_from_quantity_input(10.0, 30_000.0, true, 5),
        Some(0.00033)
    );
}

#[test]
fn order_size_rejects_size_below_asset_precision() {
    assert_eq!(
        order_size_from_quantity_input(10.0, 30_000.0, true, 2),
        None
    );
    assert_eq!(order_size_from_quantity_input(0.009, 100.0, false, 2), None);
}

#[test]
fn order_size_rejects_invalid_raw_quantity_or_conversion_price() {
    assert_eq!(order_size_from_quantity_input(0.0, 100.0, false, 5), None);
    assert_eq!(
        order_size_from_quantity_input(f64::NAN, 100.0, false, 5),
        None
    );
    assert_eq!(order_size_from_quantity_input(250.0, 0.0, true, 5), None);
    assert_eq!(
        order_size_from_quantity_input(250.0, f64::INFINITY, true, 5),
        None
    );
}
