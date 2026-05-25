use super::*;

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
