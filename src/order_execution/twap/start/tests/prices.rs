use super::*;

#[test]
fn parse_positive_price_rejects_invalid_nonpositive_or_nonfinite_values() {
    assert_eq!(parse_positive_price("100.5"), Some(100.5));
    assert_eq!(parse_positive_price("0"), None);
    assert_eq!(parse_positive_price("-1"), None);
    assert_eq!(parse_positive_price("NaN"), None);
    assert_eq!(parse_positive_price("bad"), None);
}
