use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use std::collections::HashSet;

#[test]
fn muted_ticker_matching_covers_plain_builder_and_universe_aliases() {
    let muted = HashSet::from(["BTC".to_string()]);
    let symbols = Vec::new();

    assert!(TradingTerminal::key_matches_muted_tickers(
        &symbols, &muted, "BTC"
    ));
    assert!(TradingTerminal::key_matches_muted_tickers(
        &symbols, &muted, "UBTC"
    ));
    assert!(TradingTerminal::key_matches_muted_tickers(
        &symbols, &muted, "flx:BTC"
    ));
    assert!(TradingTerminal::key_matches_muted_tickers(
        &symbols, &muted, "flx:UBTC"
    ));
    assert!(!TradingTerminal::key_matches_muted_tickers(
        &symbols, &muted, "ETH"
    ));
}

#[test]
fn muted_ticker_matching_uses_exchange_symbol_metadata_for_spot_keys() {
    let muted = HashSet::from(["HYPE".to_string()]);
    let symbols = vec![ExchangeSymbol {
        key: "@107".to_string(),
        ticker: "HYPE".to_string(),
        category: "spot".to_string(),
        display_name: Some("HYPE/USDC".to_string()),
        keywords: Vec::new(),
        asset_index: 10107,
        sz_decimals: 2,
        max_leverage: 0,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }];

    assert!(TradingTerminal::key_matches_muted_tickers(
        &symbols, &muted, "@107"
    ));
}
