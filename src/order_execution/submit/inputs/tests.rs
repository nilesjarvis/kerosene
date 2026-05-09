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

#[test]
fn order_quantity_keeps_coin_amount_when_not_denominated_in_usd() {
    assert_eq!(order_quantity_from_input(2.5, 100.0, false), Some(2.5));
}

#[test]
fn order_quantity_converts_usd_amount_by_price() {
    assert_eq!(order_quantity_from_input(250.0, 100.0, true), Some(2.5));
}

#[test]
fn order_quantity_rejects_invalid_raw_quantity_or_conversion_price() {
    assert_eq!(order_quantity_from_input(0.0, 100.0, false), None);
    assert_eq!(order_quantity_from_input(f64::NAN, 100.0, false), None);
    assert_eq!(order_quantity_from_input(250.0, 0.0, true), None);
    assert_eq!(order_quantity_from_input(250.0, f64::INFINITY, true), None);
}
