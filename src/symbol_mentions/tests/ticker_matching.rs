use super::*;
use crate::api::{ExchangeSymbol, MarketType};

fn symbol(key: &str, ticker: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn pairs(mentions: &[SymbolMention]) -> Vec<(String, String)> {
    mentions
        .iter()
        .map(|m| (m.symbol_key.clone(), m.ticker.clone()))
        .collect()
}

#[test]
fn resolves_bare_and_cashtag_ticker_mentions() {
    let symbols = vec![
        symbol("BTC", "BTC", MarketType::Perp),
        symbol("ETH", "ETH", MarketType::Perp),
        symbol("xyz:NVDA", "NVDA", MarketType::Perp),
    ];

    let mentions = resolve_symbol_mentions("BTC bounced while $eth and NVDA lagged", &symbols);

    assert_eq!(
        pairs(&mentions),
        vec![
            ("BTC".to_string(), "BTC".to_string()),
            ("ETH".to_string(), "ETH".to_string()),
            ("xyz:NVDA".to_string(), "NVDA".to_string()),
        ]
    );
    assert_eq!(mentions[1].matched_text, "eth");
    assert_eq!(mentions[1].source, SymbolAliasSource::Ticker);
}

#[test]
fn avoids_substrings_and_weak_short_words() {
    let symbols = vec![
        symbol("BTC", "BTC", MarketType::Perp),
        symbol("IN", "IN", MarketType::Perp),
        symbol("ME", "ME", MarketType::Perp),
        symbol("S", "S", MarketType::Perp),
    ];

    let mentions =
        resolve_symbol_mentions("bitcoin is in motion; s curve; $me too, BTC.", &symbols);

    assert_eq!(
        pairs(&mentions),
        vec![
            ("ME".to_string(), "ME".to_string()),
            ("BTC".to_string(), "BTC".to_string()),
        ]
    );
}

#[test]
fn cashtags_still_match_weak_single_letter_words() {
    let symbols = vec![symbol("S", "S", MarketType::Perp)];

    let mentions = resolve_symbol_mentions("$s squeeze", &symbols);

    assert_eq!(pairs(&mentions), vec![("S".to_string(), "S".to_string())]);
}

#[test]
fn avoids_single_letter_tickers_in_apostrophe_words() {
    let symbols = vec![symbol("S", "S", MarketType::Perp)];

    let lowercase = resolve_symbol_mentions("it's still early", &symbols);
    let uppercase = resolve_symbol_mentions("IT'S still early and BTC'S dominance held", &symbols);

    assert!(lowercase.is_empty());
    assert!(uppercase.is_empty());
}

#[test]
fn avoids_ambiguous_lowercase_bare_ticker_words() {
    let symbols = vec![
        symbol("LINK", "LINK", MarketType::Perp),
        symbol("NEAR", "NEAR", MarketType::Perp),
        symbol("APT", "APT", MarketType::Perp),
    ];

    let lowercase = resolve_symbol_mentions(
        "people shared a link near the venue with apt timing",
        &symbols,
    );
    let strong = resolve_symbol_mentions("LINK and $near moved with APT", &symbols);

    assert!(lowercase.is_empty());
    assert_eq!(
        pairs(&strong),
        vec![
            ("LINK".to_string(), "LINK".to_string()),
            ("NEAR".to_string(), "NEAR".to_string()),
            ("APT".to_string(), "APT".to_string()),
        ]
    );
}

#[test]
fn prefers_perp_when_tickers_overlap() {
    let symbols = vec![
        symbol("@107", "HYPE", MarketType::Spot),
        symbol("HYPE", "HYPE", MarketType::Perp),
    ];

    let mentions = resolve_symbol_mentions("HYPE headline", &symbols);

    assert_eq!(
        pairs(&mentions),
        vec![("HYPE".to_string(), "HYPE".to_string())]
    );
}

#[test]
fn skips_spot_only_symbols() {
    let symbols = vec![symbol("@1", "PURR", MarketType::Spot)];

    let mentions = resolve_symbol_mentions("PURR headline", &symbols);

    assert!(mentions.is_empty());
}
