use super::*;

#[test]
fn twap_child_notional_enforces_exchange_minimum() {
    assert!(twap_order_notional_meets_minimum(
        0.1,
        MIN_EXCHANGE_ORDER_NOTIONAL_USD * 100.0
    ));
    assert!(!twap_order_notional_meets_minimum(0.009, 1_000.0));
    assert_eq!(
        twap_min_quantized_child_notional(1.0, 10, 100.0, false, 3),
        Some(10.0)
    );
    let randomized = positive_child_notional(1.0, 10, 100.0, true, 3);
    assert!((randomized - 8.0).abs() < 1e-9);
}

#[test]
fn twap_child_notional_uses_quantized_child_size() {
    assert_eq!(
        twap_min_quantized_child_notional(9.9, 10, 11.0, false, 0),
        None
    );
    assert_eq!(
        twap_min_quantized_child_notional(10.9, 10, 10.0, false, 0),
        Some(10.0)
    );
}
