use super::super::twap_ioc_limit_price;

#[test]
fn twap_ioc_limit_price_preserves_marketability_after_rounding() {
    assert_eq!(
        twap_ioc_limit_price(1.2344, true, 3, false, 1.0, 2.0),
        Some(1.2344)
    );
    assert_eq!(
        twap_ioc_limit_price(1.2346, false, 3, false, 1.0, 2.0),
        Some(1.2346)
    );
    assert_eq!(
        twap_ioc_limit_price(100.0, true, 3, false, 99.0, 100.0),
        Some(100.0)
    );
    assert_eq!(
        twap_ioc_limit_price(100.1, true, 3, false, 99.0, 100.0),
        None
    );
}
