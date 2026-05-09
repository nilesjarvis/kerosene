use super::*;

fn symbol(key: &str, ticker: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: String::new(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        sz_decimals: 0,
        max_leverage: 0,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

#[test]
fn exact_key_takes_precedence_over_ticker_fallback() {
    let symbols = vec![
        symbol("MAIN", "ALT", MarketType::Perp),
        symbol("ALT", "MAIN", MarketType::Spot),
    ];

    let resolved = resolve_exchange_symbol(&symbols, "ALT").expect("symbol");

    assert_eq!(resolved.key, "ALT");
    assert_eq!(resolved.market_type, MarketType::Spot);
}

#[test]
fn ticker_fallback_prefers_perp_market() {
    let symbols = vec![
        symbol("@107", "HYPE", MarketType::Spot),
        symbol("HYPE", "HYPE", MarketType::Perp),
    ];

    let resolved = resolve_exchange_symbol(&symbols, "HYPE").expect("symbol");

    assert_eq!(resolved.key, "HYPE");
    assert_eq!(resolved.market_type, MarketType::Perp);
}

#[test]
fn ticker_fallback_returns_non_perp_when_no_perp_exists() {
    let symbols = vec![
        symbol("#0", "YES", MarketType::Outcome),
        symbol("@1", "PURR", MarketType::Spot),
    ];

    let resolved = resolve_exchange_symbol(&symbols, "PURR").expect("symbol");

    assert_eq!(resolved.key, "@1");
    assert_eq!(resolved.market_type, MarketType::Spot);
}
