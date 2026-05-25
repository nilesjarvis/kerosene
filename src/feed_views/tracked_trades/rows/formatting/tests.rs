use super::*;
use crate::config::DisplayDenominationConfig;
use std::collections::HashMap;

#[test]
fn tracked_trade_numbers_drop_empty_decimal_places() {
    assert_eq!(tracked_trade_size_label(2.0), "2");
    assert_eq!(tracked_trade_price_label(12_345.0), "12345");
    assert_eq!(tracked_trade_fee_label(1.0, "USDC"), "1 USDC");
}

#[test]
fn tracked_trade_numbers_keep_meaningful_decimal_places() {
    assert_eq!(tracked_trade_size_label(2.5), "2.5");
    assert_eq!(tracked_trade_price_label(12_345.6789), "12345.6789");
    assert_eq!(tracked_trade_fee_label(0.0123, "USDC"), "0.0123 USDC");
}

#[test]
fn tracked_trade_usd_values_trim_zero_cents() {
    let denomination = DisplayDenominationContext::default();
    assert_eq!(
        tracked_trade_notional_label(&denomination, 12_000.0),
        "$12,000"
    );
    assert_eq!(tracked_trade_pnl_label(&denomination, 12.0), "+$12");
    assert_eq!(tracked_trade_pnl_label(&denomination, -12.5), "-$12.5");
    assert_eq!(
        tracked_trade_notional_label(&denomination, 1_500_000.0),
        "$1.5M"
    );
}

#[test]
fn tracked_trade_hype_values_suffix_unit() {
    let denomination = DisplayDenominationContext::from_mids(
        DisplayDenominationConfig::hype(),
        &HashMap::from([("HYPE".to_string(), 25.0)]),
        &HashMap::from([("HYPE".to_string(), 1_000)]),
        1_000,
    );

    assert_eq!(
        tracked_trade_notional_label(&denomination, 12_500.0),
        "500 HYPE"
    );
    assert_eq!(
        tracked_trade_pnl_label(&denomination, -2_500.0),
        "-100 HYPE"
    );
}
