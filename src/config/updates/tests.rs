use super::*;

#[test]
fn updates_decimal_normalization_preserves_precision_and_numeric_order() {
    let decimal = |s: &str| ParameterDecimal::try_from(s.to_string()).expect("decimal");
    assert_eq!(decimal("000100.000"), decimal("100"));
    assert_eq!(decimal("000.00100").as_str(), "0.001");
    assert!(decimal("9.9") < decimal("10"));
    assert!(decimal("1.00000000000000001") > decimal("1"));
    assert!(decimal("100000000000000000001") > decimal("100000000000000000000"));
    for invalid in ["", "NaN", "inf", "-1", "+1", "1e10", " 1", ".1", "1.2.3"] {
        assert!(
            ParameterDecimal::try_from(invalid.to_string()).is_err(),
            "{invalid}"
        );
    }
    let value = decimal("100.50");
    let wire = serde_json::to_string(&value).expect("encode");
    assert_eq!(wire, "\"100.5\"");
    assert_eq!(
        serde_json::from_str::<ParameterDecimal>(&wire).expect("decode"),
        value
    );
}
