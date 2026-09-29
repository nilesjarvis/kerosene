use super::*;

#[test]
fn pnl_card_window_state_debug_redacts_account_address() {
    let state = PnlCardWindowState::new(PnlCardTarget::Position("BTC".to_string()), test_account());

    let rendered = format!("{state:?}");

    assert!(!rendered.contains(&test_account()));
    assert!(rendered.contains("<redacted>"));
    assert!(rendered.contains("Position(\"BTC\")"));
    assert!(rendered.contains("obscure_prices: true"));
}

#[test]
fn privacy_price_display_can_be_disabled() {
    assert_eq!(privacy_price_display("82,543.2", true), "82,5xx");
    assert_eq!(privacy_price_display("82,543.2", false), "82,543.2");
}

#[test]
fn price_privacy_obscures_large_prices_to_hundreds() {
    assert_eq!(obscure_price_digits("82,543.2"), "82,5xx");
    assert_eq!(obscure_price_digits("12,345.7"), "12,3xx");
    assert_eq!(obscure_price_digits("-12,345.7"), "-12,3xx");
    assert_eq!(obscure_price_digits("1,234.5"), "1,2xx");
}

#[test]
fn price_privacy_scales_across_mid_price_denominations() {
    assert_eq!(obscure_price_digits("825.42"), "82x");
    assert_eq!(obscure_price_digits("82.54"), "8x");
    assert_eq!(obscure_price_digits("8.254"), "8.xxx");
    assert_eq!(obscure_price_digits("8"), "x");
}

#[test]
fn price_privacy_keeps_only_early_significant_sub_dollar_digits() {
    assert_eq!(obscure_price_digits("0.123456"), "0.1xxxxx");
    assert_eq!(obscure_price_digits("0.012345"), "0.01xxxx");
    assert_eq!(obscure_price_digits("0.00001234"), "0.00001xxx");
    assert_eq!(obscure_price_digits("0.0000"), "0.00xx");
}

#[test]
fn price_privacy_preserves_signs_separators_and_existing_non_digit_rules() {
    for (price, expected) in [
        ("", ""),
        (" \t", " \t"),
        ("  +12,345.67  ", "+12,3xx"),
        ("-82.54", "-8x"),
        ("€12.34", "€1x"),
        ("1\u{202f}234.5", "1\u{202f}2xx"),
        ("0.a012", "0.a0xx"),
        ("0.é0001", "0.é00xx"),
        ("0.abc", "0.abc"),
        ("0.1", "0.x"),
        ("0.01", "0.xx"),
        ("8.€", "8.xxx"),
        ("１２.3456", "１２.3xxx"),
        ("n/a", "x"),
    ] {
        assert_eq!(obscure_price_digits(price), expected, "{price}");
        assert_eq!(privacy_price_display(price, false), price);
    }
}
