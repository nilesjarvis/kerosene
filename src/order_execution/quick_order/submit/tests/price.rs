use super::*;

#[test]
fn quick_order_limit_price_wire_rejects_invalid_or_zero_rounded_prices() {
    assert_eq!(quick_order_limit_price_wire(f64::NAN, 2, false), None);
    assert_eq!(quick_order_limit_price_wire(f64::INFINITY, 2, false), None);
    assert_eq!(quick_order_limit_price_wire(0.0, 2, false), None);
    assert_eq!(quick_order_limit_price_wire(-1.0, 2, false), None);
    assert_eq!(quick_order_limit_price_wire(0.0000001, 2, false), None);
}

#[test]
fn quick_order_limit_price_wire_returns_rounded_wire_price() {
    assert_eq!(
        quick_order_limit_price_wire(123.456789, 2, false),
        Some((123.46, "123.46".into()))
    );
}

#[test]
fn quick_order_market_price_wire_rejects_invalid_or_zero_prices() {
    assert_eq!(
        quick_order_market_price_wire(f64::NAN, true, 0.05, 2, false),
        None
    );
    assert_eq!(
        quick_order_market_price_wire(f64::INFINITY, true, 0.05, 2, false),
        None
    );
    assert_eq!(
        quick_order_market_price_wire(0.0000001, true, 0.05, 2, false),
        None
    );
}

#[test]
fn quick_order_market_price_wire_applies_slippage_and_rounding() {
    assert_eq!(
        quick_order_market_price_wire(100.0, true, 0.05, 2, false),
        Some((105.0, "105".into()))
    );
    assert_eq!(
        quick_order_market_price_wire(100.0, false, 0.05, 2, false),
        Some((95.0, "95".into()))
    );
}
