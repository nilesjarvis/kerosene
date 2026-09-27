use super::{outcome_price_text, outcome_probability_text};

#[test]
fn scalar_prices_are_quote_token_values_instead_of_probabilities() {
    assert_eq!(outcome_price_text(Some(0.42), true, "USDH"), "0.420 USDH");
    assert_eq!(outcome_price_text(Some(0.42), false, "USDH"), "42.0%");
    assert_eq!(outcome_price_text(Some(1.01), true, "USDC"), "n/a");
}

#[test]
fn outcome_probability_text_uses_not_available_placeholder_for_invalid_mid() {
    assert_eq!(outcome_probability_text(Some(0.42)), "42.0%");
    assert_eq!(outcome_probability_text(None), "n/a");
    assert_eq!(outcome_probability_text(Some(f64::NAN)), "n/a");
    assert_eq!(outcome_probability_text(Some(f64::INFINITY)), "n/a");
    assert_eq!(outcome_probability_text(Some(-0.1)), "n/a");
    assert_eq!(outcome_probability_text(Some(1.1)), "n/a");
}
