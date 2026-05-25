use super::*;

#[test]
fn number_parser_accepts_grouped_values() {
    assert_eq!(parse_number("12,345.67"), Some(12_345.67));
    assert_eq!(parse_number("1,234,567"), Some(1_234_567.0));
    assert_eq!(parse_number("-1,234.50"), Some(-1_234.5));
    assert_eq!(parse_number("+1,234.50"), Some(1_234.5));
    assert_eq!(parse_number(""), None);
}

#[test]
fn number_parser_rejects_malformed_grouped_values() {
    assert_eq!(parse_number("1,2"), None);
    assert_eq!(parse_number("1,,000"), None);
    assert_eq!(parse_number("12,34.56"), None);
    assert_eq!(parse_number("1234,567"), None);
    assert_eq!(parse_number(",123"), None);
    assert_eq!(parse_number("1,234.5,6"), None);
    assert_eq!(parse_number("1,234e2"), None);
}

#[test]
fn number_parser_rejects_nonfinite_values() {
    assert_eq!(parse_number("NaN"), None);
    assert_eq!(parse_number("inf"), None);
    assert_eq!(parse_number("1e309"), None);
}

#[test]
fn positive_number_parser_preserves_grouped_ui_number_rules() {
    assert_eq!(parse_positive_number("1,234.50"), Some(1_234.5));
    assert_eq!(parse_positive_number("0"), None);
    assert_eq!(parse_positive_number("-1"), None);
    assert_eq!(parse_positive_number("NaN"), None);
    assert_eq!(parse_positive_number("1,23"), None);
}
