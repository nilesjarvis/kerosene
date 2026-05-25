use super::super::moved_order_price_wire;

#[test]
fn moved_order_price_returns_none_when_rounded_price_is_unchanged() {
    assert_eq!(moved_order_price_wire(100.001, 100.0, 2, false), None);
}

#[test]
fn moved_order_price_returns_rounded_value_and_wire_price_when_rounded_price_changes() {
    assert_eq!(
        moved_order_price_wire(101.0, 100.0, 2, false),
        Some((101.0, "101".to_string()))
    );
}

#[test]
fn moved_order_price_rejects_nonfinite_new_price() {
    assert_eq!(moved_order_price_wire(f64::NAN, 100.0, 2, false), None);
    assert_eq!(moved_order_price_wire(f64::INFINITY, 100.0, 2, false), None);
}

#[test]
fn moved_order_price_rejects_invalid_original_price() {
    assert_eq!(moved_order_price_wire(101.0, f64::NAN, 2, false), None);
    assert_eq!(moved_order_price_wire(101.0, 0.0, 2, false), None);
    assert_eq!(moved_order_price_wire(101.0, -1.0, 2, false), None);
}

#[test]
fn moved_order_price_rejects_non_positive_rounded_new_price() {
    assert_eq!(moved_order_price_wire(0.0, 100.0, 2, false), None);
    assert_eq!(moved_order_price_wire(-1.0, 100.0, 2, false), None);
    assert_eq!(moved_order_price_wire(0.0000001, 100.0, 2, false), None);
}
