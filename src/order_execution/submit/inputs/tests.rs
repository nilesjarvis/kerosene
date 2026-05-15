use super::*;

#[test]
fn positive_amount_parser_accepts_trimmed_positive_numbers() {
    assert_eq!(parse_positive_amount(" 12.5 "), Some(12.5));
}

#[test]
fn positive_amount_parser_accepts_grouped_numbers() {
    assert_eq!(parse_positive_amount("12,345.67"), Some(12_345.67));
}

#[test]
fn positive_amount_parser_rejects_malformed_grouped_numbers() {
    assert_eq!(parse_positive_amount("1,2"), None);
    assert_eq!(parse_positive_amount("1,,000"), None);
    assert_eq!(parse_positive_amount("12,34.56"), None);
}

#[test]
fn positive_amount_parser_rejects_zero_negative_and_nonfinite_values() {
    assert_eq!(parse_positive_amount("0"), None);
    assert_eq!(parse_positive_amount("-1"), None);
    assert_eq!(parse_positive_amount("NaN"), None);
    assert_eq!(parse_positive_amount("inf"), None);
    assert_eq!(parse_positive_amount("nope"), None);
}
