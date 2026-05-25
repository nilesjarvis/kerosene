use super::*;

#[test]
fn account_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_account_number(" 123.45 "), Some(123.45));
    assert_eq!(parse_account_number("-0.5"), Some(-0.5));

    assert_eq!(parse_account_number("bad"), None);
    assert_eq!(parse_account_number("NaN"), None);
    assert_eq!(parse_account_number("inf"), None);
}
