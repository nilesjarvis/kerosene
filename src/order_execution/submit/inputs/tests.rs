use super::*;

#[test]
fn positive_amount_parser_accepts_trimmed_positive_numbers() {
    assert_eq!(parse_positive_amount(" 12.5 "), Some(12.5));
}

#[test]
fn positive_amount_parser_rejects_zero_negative_and_nonfinite_values() {
    assert_eq!(parse_positive_amount("0"), None);
    assert_eq!(parse_positive_amount("-1"), None);
    assert_eq!(parse_positive_amount("NaN"), None);
    assert_eq!(parse_positive_amount("inf"), None);
    assert_eq!(parse_positive_amount("nope"), None);
}
