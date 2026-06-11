use super::calculations::{order_notional_text, parse_positive_finite};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;

fn spot_symbol(key: &str, display_name: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "HYPE".to_string(),
        category: "spot".to_string(),
        display_name: Some(display_name.to_string()),
        keywords: Vec::new(),
        asset_index: 10_107,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

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
        order_notional_text(true, "BTC", false, Some(100.0), None),
        (Some(100.0), String::new())
    );
}

#[test]
fn coin_quantity_requires_valid_reference_price_for_notional() {
    assert_eq!(
        order_notional_text(false, "BTC", false, Some(2.0), None),
        (None, String::new())
    );
    assert_eq!(
        order_notional_text(false, "BTC", false, Some(2.0), Some(125.0)),
        (Some(250.0), "\u{2248} $250.00".to_string())
    );
}

#[test]
fn usd_quantity_coin_text_uses_resolved_spot_pair_name_not_raw_key() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE/USDC")];
    let display = terminal.display_name_for_symbol("@107");

    assert_eq!(
        order_notional_text(true, &display, false, Some(100.0), Some(25.0)),
        (Some(100.0), "\u{2248} 4.0000 HYPE/USDC".to_string())
    );
}

#[test]
fn usd_quantity_coin_text_uses_whole_contracts_for_outcome_symbols() {
    assert_eq!(
        order_notional_text(true, "YES: BTC above 75348", true, Some(10.0), Some(0.5)),
        (Some(10.0), "\u{2248} 20 YES: BTC above 75348".to_string())
    );
}
