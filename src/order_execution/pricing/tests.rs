use super::*;

const DEFAULT_MARKET_SLIPPAGE: f64 = DEFAULT_MARKET_SLIPPAGE_PCT / 100.0;

#[test]
fn slipped_market_price_moves_buy_up_and_sell_down() {
    assert_eq!(
        slipped_market_price(100.0, true, DEFAULT_MARKET_SLIPPAGE),
        101.0
    );
    assert_eq!(
        slipped_market_price(100.0, false, DEFAULT_MARKET_SLIPPAGE),
        99.0
    );
}

#[test]
fn rounded_market_price_rounds_after_slippage() {
    assert_eq!(
        rounded_market_price(100.0, true, DEFAULT_MARKET_SLIPPAGE, 2, false),
        101.0
    );
    assert_eq!(
        rounded_market_price(100.0, false, DEFAULT_MARKET_SLIPPAGE, 2, false),
        99.0
    );
}

#[test]
fn wire_helpers_strip_unneeded_decimal_zeros() {
    assert_eq!(
        wire_market_price(100.0, true, DEFAULT_MARKET_SLIPPAGE, 2, false),
        "101"
    );
    assert_eq!(wire_rounded_price(95.0, 2, false), "95");
}

#[test]
fn market_slippage_pct_accepts_bounded_finite_percentages() {
    assert_eq!(normalize_market_slippage_pct(0.0), Some(0.0));
    assert_eq!(normalize_market_slippage_pct(1.25), Some(1.25));
    assert_eq!(
        normalize_market_slippage_pct(MAX_MARKET_SLIPPAGE_PCT),
        Some(MAX_MARKET_SLIPPAGE_PCT)
    );
}

#[test]
fn market_slippage_pct_rejects_negative_nonfinite_or_too_large_values() {
    assert_eq!(normalize_market_slippage_pct(-0.1), None);
    assert_eq!(normalize_market_slippage_pct(f64::NAN), None);
    assert_eq!(normalize_market_slippage_pct(f64::INFINITY), None);
    assert_eq!(
        normalize_market_slippage_pct(MAX_MARKET_SLIPPAGE_PCT + 0.1),
        None
    );
}

#[test]
fn market_slippage_fraction_falls_back_to_default_for_invalid_config() {
    assert_eq!(market_slippage_fraction(1.5), 0.015);
    assert_eq!(
        market_slippage_fraction(f64::NAN),
        DEFAULT_MARKET_SLIPPAGE_PCT / 100.0
    );
}
