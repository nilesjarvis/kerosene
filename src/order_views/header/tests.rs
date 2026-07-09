use super::*;

#[test]
fn order_header_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_order_header_number(" 12.5 "), Some(12.5));
    assert_eq!(parse_order_header_number("-3"), Some(-3.0));
    assert_eq!(parse_order_header_number("bad"), None);
    assert_eq!(parse_order_header_number("NaN"), None);
    assert_eq!(parse_order_header_number("inf"), None);
}

#[test]
fn order_header_usd_formatter_marks_invalid_values() {
    assert_eq!(format_optional_usd(Some(1234.5)), "$1,234.50");
    assert_eq!(format_optional_usd(None), "Invalid data");
}

#[test]
fn order_header_non_usd_quote_formatter_never_adds_a_dollar_sign() {
    assert_eq!(
        format_optional_token_amount(Some(1.2345), Some("UBTC")),
        "1.2345 UBTC"
    );
    assert_eq!(
        format_optional_token_amount(Some(1.0), None),
        "Invalid data"
    );
}

#[test]
fn order_leverage_labels_preserve_actual_and_max_modes() {
    assert_eq!(order_leverage_label(true, 10, true), "Cross 10x");
    assert_eq!(order_leverage_label(false, 5, true), "Isolated 5x");
    assert_eq!(order_leverage_label(false, 50, false), "Max 50x");
}
