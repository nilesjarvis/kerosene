use super::*;

#[test]
fn quick_order_size_wire_converts_usd_notional_to_coin_size() {
    assert_eq!(
        quick_order_size_wire("250", true, 100.0, 5),
        Some("2.5".into())
    );
    assert_eq!(
        quick_order_size_wire("2.5", false, 100.0, 5),
        Some("2.5".into())
    );
    assert_eq!(
        quick_order_size_wire("1,234.5", false, 100.0, 5),
        Some("1234.5".into())
    );
}

#[test]
fn quick_order_size_wire_quantizes_to_asset_precision() {
    assert_eq!(
        quick_order_size_wire("10", true, 30_000.0, 5),
        Some("0.00033".into())
    );
    assert_eq!(
        quick_order_size_wire("1.239", false, 100.0, 2),
        Some("1.23".into())
    );
}

#[test]
fn quick_order_size_wire_rejects_invalid_reference_for_usd() {
    assert_eq!(quick_order_size_wire("250", true, 0.0, 5), None);
    assert_eq!(quick_order_size_wire("250", true, f64::NAN, 5), None);
    assert_eq!(quick_order_size_wire("0", false, 100.0, 5), None);
    assert_eq!(quick_order_size_wire("-1", false, 100.0, 5), None);
    assert_eq!(quick_order_size_wire("NaN", false, 100.0, 5), None);
    assert_eq!(
        quick_order_size_wire("0.0000000000001", false, 100.0, 8),
        None
    );
    assert_eq!(quick_order_size_wire("10", true, 30_000.0, 2), None);
}
