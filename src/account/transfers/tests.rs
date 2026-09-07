use super::*;

#[test]
fn exact_native_units_preserve_wei_satoshis_and_fractional_fee_estimates() {
    for (raw, decimals, expected) in [
        ("10000000", 8, "0.1"),
        ("100000000000000001", 18, "0.100000000000000001"),
        ("140761861078.125", 18, "0.000000140761861078125"),
        ("1", 18, "0.000000000000000001"),
        ("100.1200", 0, "100.12"),
        ("0", 8, "0"),
    ] {
        assert_eq!(decimal_units(raw, decimals).as_deref(), Some(expected));
    }
    for raw in ["NaN", "-1", "1e8", "1.2.3", "", ".1"] {
        assert!(decimal_units(raw, 8).is_none());
    }
}
