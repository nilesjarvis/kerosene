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
fn bare_lowercase_common_word_tickers_do_not_match() {
    let symbols = vec![
        symbol("PUMP", "PUMP", MarketType::Perp),
        symbol("PEOPLE", "PEOPLE", MarketType::Perp),
        symbol("BAN", "BAN", MarketType::Perp),
        symbol("MOVE", "MOVE", MarketType::Perp),
        symbol("BTC", "BTC", MarketType::Perp),
    ];

    // Ordinary English prose must never light up tickers that happen to be common
    // words. Before the strong-match gate, 3+ char word tickers matched bare
    // lowercase text and showed a live price-impact chip next to unrelated news.
    let mentions = resolve_symbol_mentions(
        "a lot of people are bullish and this will pump soon unless they move to ban it",
        &symbols,
    );

    assert!(
        mentions.is_empty(),
        "expected no mentions, got {mentions:?}"
    );
}

#[test]
fn uppercase_common_word_ticker_still_matches_in_normal_text() {
    let symbols = vec![symbol("PUMP", "PUMP", MarketType::Perp)];

    // Uppercase in ordinary (non-headline) prose is still a deliberate signal.
    let mentions = resolve_symbol_mentions("desk thinks PUMP rips after the unlock", &symbols);

    assert_eq!(
        pairs(&mentions),
        vec![("PUMP".to_string(), "PUMP".to_string())]
    );
}

#[test]
fn all_caps_headline_suppresses_ambiguous_words_but_keeps_real_tickers() {
    let symbols = vec![
        symbol("GO", "GO", MarketType::Perp),
        symbol("ON", "ON", MarketType::Perp),
        symbol("PEOPLE", "PEOPLE", MarketType::Perp),
        symbol("BTC", "BTC", MarketType::Perp),
    ];

    // In an all-caps headline uppercase carries no signal, so common words must
    // not resolve; only the genuine ticker BTC should.
    let mentions = resolve_symbol_mentions(
        "BREAKING MARKETS GO WILD AS PEOPLE PILE ON AND BTC RIPS HIGHER TODAY",
        &symbols,
    );

    assert_eq!(
        pairs(&mentions),
        vec![("BTC".to_string(), "BTC".to_string())]
    );
}

#[test]
fn cashtag_overrides_all_caps_headline_suppression() {
    let symbols = vec![symbol("GO", "GO", MarketType::Perp)];

    // An explicit cashtag is a deliberate reference even inside a shouty headline.
    let mentions = resolve_symbol_mentions(
        "BREAKING TRADERS PILE INTO $GO ACROSS GLOBAL MARKETS THIS MORNING",
        &symbols,
    );

    assert_eq!(pairs(&mentions), vec![("GO".to_string(), "GO".to_string())]);
}

#[test]
fn explicit_ticker_survives_nested_keyword_phrase() {
    let mut ldo = symbol("LDO", "LDO", MarketType::Perp);
    ldo.keywords = vec!["eth staking".to_string()];
    let symbols = vec![ldo, symbol("ETH", "ETH", MarketType::Perp)];

    let mentions = resolve_symbol_mentions("ETH staking inflows grew this week", &symbols);

    // The explicitly-typed ETH must not be hidden by LDO's "eth staking" keyword;
    // both are legitimate, distinct symbols.
    assert_eq!(
        pairs(&mentions),
        vec![
            ("ETH".to_string(), "ETH".to_string()),
            ("LDO".to_string(), "LDO".to_string()),
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
