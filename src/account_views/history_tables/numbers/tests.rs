use super::*;

#[test]
fn history_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_history_number(" 1.25 "), Some(1.25));
    assert_eq!(parse_history_number("-2"), Some(-2.0));

    assert_eq!(parse_history_number("bad"), None);
    assert_eq!(parse_history_number("NaN"), None);
    assert_eq!(parse_history_number("inf"), None);
}

#[test]
fn history_wire_values_are_validated_without_reformatting() {
    assert_eq!(valid_history_wire_value("123.4500"), "123.4500");
    assert_eq!(valid_history_wire_value("NaN"), "Invalid data");
}

#[test]
fn history_usd_formatter_marks_invalid_values() {
    assert_eq!(format_history_usd(Some(12.5), 2), "$12.50");
    assert_eq!(format_history_usd(None, 2), "Invalid data");
}
