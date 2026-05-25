use super::calculations::{order_notional_text, parse_positive_finite};

#[test]
fn size_input_parser_rejects_invalid_nonpositive_or_nonfinite_values() {
    assert_eq!(parse_positive_finite("12.5"), Some(12.5));
    assert_eq!(parse_positive_finite("1,234.5"), Some(1_234.5));
    assert_eq!(parse_positive_finite("0"), None);
    assert_eq!(parse_positive_finite("-1"), None);
    assert_eq!(parse_positive_finite("NaN"), None);
    assert_eq!(parse_positive_finite("bad"), None);
}

#[test]
fn usd_quantity_keeps_known_notional_when_price_is_missing() {
    assert_eq!(
        order_notional_text(true, "BTC", Some(100.0), None),
        (Some(100.0), String::new())
    );
}

#[test]
fn coin_quantity_requires_valid_reference_price_for_notional() {
    assert_eq!(
        order_notional_text(false, "BTC", Some(2.0), None),
        (None, String::new())
    );
    assert_eq!(
        order_notional_text(false, "BTC", Some(2.0), Some(125.0)),
        (Some(250.0), "\u{2248} $250.00".to_string())
    );
}
