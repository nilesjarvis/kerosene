use super::*;

#[test]
fn wallet_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_wallet_number(" 12.5 "), Some(12.5));
    assert_eq!(parse_wallet_number("-3"), Some(-3.0));

    assert_eq!(parse_wallet_number("bad"), None);
    assert_eq!(parse_wallet_number("NaN"), None);
    assert_eq!(parse_wallet_number("inf"), None);
}

#[test]
fn wallet_visible_nonzero_keeps_invalid_values_visible() {
    assert!(wallet_has_visible_nonzero("bad"));
    assert!(wallet_has_visible_nonzero("1"));
    assert!(wallet_has_visible_nonzero("-1"));
    assert!(!wallet_has_visible_nonzero("0"));
}

#[test]
fn wallet_formatters_mark_invalid_values() {
    assert_eq!(format_wallet_usd(Some(12.5), 2), "$12.50");
    assert_eq!(format_wallet_usd(None, 2), "Invalid data");
    assert_eq!(format_wallet_price(None), "Invalid data");
    assert_eq!(format_wallet_amount(Some(2.5), false), "2.500000");
    assert_eq!(format_wallet_amount(None, true), "Invalid data");
}
