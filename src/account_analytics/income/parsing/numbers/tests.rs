use super::*;

#[test]
fn income_number_parser_keeps_plain_finite_wire_rules() {
    assert_eq!(parse_f64_str(" 12.5 "), Some(12.5));
    assert_eq!(parse_f64_str("-3"), Some(-3.0));
    assert_eq!(parse_f64_str("NaN"), None);
    assert_eq!(parse_f64_str("inf"), None);
    assert_eq!(parse_f64_str("1,234"), None);
}
