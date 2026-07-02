use super::*;

fn symbol(key: &str, ticker: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: String::new(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
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

#[test]
fn legacy_indexed_key_resolves_api_named_spot_pair() {
    // PURR/USDC is keyed by its API name; layouts saved before the re-key
    // still carry "@0" and must migrate to the canonical key on load.
    let mut purr = symbol("PURR/USDC", "PURR", MarketType::Spot);
    purr.asset_index = 10_000;
    let symbols = vec![purr, symbol("HYPE", "HYPE", MarketType::Perp)];

    let resolved = resolve_exchange_symbol(&symbols, "@0").expect("symbol");

    assert_eq!(resolved.key, "PURR/USDC");
    assert_eq!(resolved.market_type, MarketType::Spot);
}
