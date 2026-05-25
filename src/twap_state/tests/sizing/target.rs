use super::*;

#[test]
fn twap_target_size_requires_fresh_reference_for_usd_quantity() {
    assert_eq!(
        twap_target_size_from_quantity(1_000.0, Some(100.0), true),
        Some(10.0)
    );
    assert_eq!(twap_target_size_from_quantity(1_000.0, None, true), None);
    assert_eq!(twap_target_size_from_quantity(2.5, None, false), Some(2.5));
    assert_eq!(
        twap_target_size_from_quantity(1_000.0, Some(0.0), true),
        None
    );
}
