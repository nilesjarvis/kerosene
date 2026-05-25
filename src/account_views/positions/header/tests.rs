use super::*;

#[test]
fn position_size_is_nonzero_rejects_invalid_and_flat_values() {
    assert!(position_size_is_nonzero("1"));
    assert!(position_size_is_nonzero("-0.5"));
    assert!(!position_size_is_nonzero("0"));
    assert!(!position_size_is_nonzero("0.0000000000001"));
    assert!(!position_size_is_nonzero("NaN"));
    assert!(!position_size_is_nonzero("inf"));
    assert!(!position_size_is_nonzero("1,234"));
}
