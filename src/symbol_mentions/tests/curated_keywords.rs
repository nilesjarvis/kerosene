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

// ---------------------------------------------------------------------------
// Keyword-to-ticker mappings
// ---------------------------------------------------------------------------

#[test]
fn curated_keywords_map_to_correct_tickers() {
    let cases: &[(&str, &str, &str, &str)] = &[
        ("hyperliquid protocol update", "HYPE", "HYPE", "hyperliquid"),
        ("zcash privacy rotation", "ZEC", "ZEC", "zcash"),
        ("sonic ecosystem update", "S", "S", "sonic"),
    ];

    for (text, expected_key, expected_ticker, expected_match) in cases {
        let sym = symbol(expected_key, expected_ticker, MarketType::Perp);
        let mentions = resolve_symbol_mentions(text, &[sym]);

        assert_eq!(
            pairs(&mentions),
            vec![(expected_key.to_string(), expected_ticker.to_string())],
            "text: {text}"
        );
        assert_eq!(mentions[0].matched_text, *expected_match, "text: {text}");
        assert_eq!(
            mentions[0].source,
            SymbolAliasSource::CuratedKeyword,
            "text: {text}"
        );
    }
}

// ---------------------------------------------------------------------------
// Oil curated rules
// ---------------------------------------------------------------------------

#[test]
fn curated_oil_rules_resolve_to_expected_symbols() {
    let symbols = vec![
        symbol("xyz:CL", "CL", MarketType::Perp),
        symbol("xyz:BRENTOIL", "BRENTOIL", MarketType::Perp),
        symbol("flx:CL", "CL", MarketType::Perp),
        symbol("flx:OIL", "OIL", MarketType::Perp),
    ];

    let cases: &[(&str, &str, &str)] = &[
        ("Crude OIL headlines", "xyz:CL", "CL"),
        ("WTI OIL bounced", "xyz:CL", "CL"),
        ("WTI crude oil bounced", "xyz:CL", "CL"),
        ("Brent OIL bounced", "xyz:BRENTOIL", "BRENTOIL"),
        ("Brent crude rally", "xyz:BRENTOIL", "BRENTOIL"),
        ("wti rally", "xyz:CL", "CL"),
        ("wti oil demand", "xyz:CL", "CL"),
        ("wti crude supply", "xyz:CL", "CL"),
        ("Iranian export risk rises", "xyz:CL", "CL"),
        ("Iran tensions", "xyz:CL", "CL"),
        ("Hormuz traffic disrupted", "xyz:CL", "CL"),
    ];

    for (text, expected_key, expected_ticker) in cases {
        let mentions = resolve_symbol_mentions(text, &symbols);
        assert_eq!(
            pairs(&mentions),
            vec![(expected_key.to_string(), expected_ticker.to_string())],
            "text: {text}"
        );
        assert!(
            mentions
                .iter()
                .all(|m| m.source == SymbolAliasSource::CuratedKeyword),
            "text: {text} — expected CuratedKeyword source"
        );
    }
}

#[test]
fn curated_oil_rules_avoid_generic_oil_false_positive() {
    let symbols = vec![
        symbol("xyz:CL", "CL", MarketType::Perp),
        symbol("xyz:BRENTOIL", "BRENTOIL", MarketType::Perp),
        symbol("flx:OIL", "OIL", MarketType::Perp),
    ];

    let mentions = resolve_symbol_mentions("Oil painting sold at auction", &symbols);

    assert!(mentions.is_empty());
}
