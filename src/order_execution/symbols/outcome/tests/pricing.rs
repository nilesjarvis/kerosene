use super::*;

#[test]
fn outcome_price_validation_uses_probability_bounds() {
    assert!(TradingTerminal::validate_outcome_order_price(OUTCOME_MIN_PRICE).is_ok());
    assert!(TradingTerminal::validate_outcome_order_price(OUTCOME_MAX_PRICE).is_ok());
    assert!(TradingTerminal::validate_outcome_order_price(0.0009).is_err());
    assert!(TradingTerminal::validate_outcome_order_price(0.9991).is_err());
    assert!(TradingTerminal::validate_outcome_order_price(f64::NAN).is_err());
}

#[test]
fn outcome_market_price_clamps_to_probability_bounds() {
    assert_eq!(
        TradingTerminal::clamp_outcome_market_price(0.0),
        OUTCOME_MIN_PRICE
    );
    assert_eq!(
        TradingTerminal::clamp_outcome_market_price(1.0),
        OUTCOME_MAX_PRICE
    );
    assert_eq!(TradingTerminal::clamp_outcome_market_price(0.42), 0.42);
}
