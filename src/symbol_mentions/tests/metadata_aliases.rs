use super::*;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};

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

fn symbol_with_metadata(
    key: &str,
    ticker: &str,
    display_name: Option<&str>,
    keywords: &[&str],
) -> ExchangeSymbol {
    let mut sym = symbol(key, ticker, MarketType::Perp);
    sym.display_name = display_name.map(str::to_string);
    sym.keywords = keywords.iter().map(|kw| kw.to_string()).collect();
    sym
}

fn pairs(mentions: &[SymbolMention]) -> Vec<(String, String)> {
    mentions
        .iter()
        .map(|m| (m.symbol_key.clone(), m.ticker.clone()))
        .collect()
}

#[test]
fn display_names_and_keywords_resolve_as_lower_confidence_aliases() {
    let symbols = vec![symbol_with_metadata(
        "xyz:NVDA",
        "NVDA",
        Some("Nvidia"),
        &["graphics chips"],
    )];

    let display_name_mentions = resolve_symbol_mentions("nvidia headlines crossed", &symbols);
    let keyword_mentions = resolve_symbol_mentions("graphics chips demand spiked", &symbols);

    assert_eq!(
        pairs(&display_name_mentions),
        vec![("xyz:NVDA".to_string(), "NVDA".to_string())]
    );
    assert_eq!(display_name_mentions[0].matched_text, "nvidia");
    assert_eq!(
        display_name_mentions[0].source,
        SymbolAliasSource::DisplayName
    );
    assert_eq!(display_name_mentions[0].confidence, 78);

    assert_eq!(
        pairs(&keyword_mentions),
        vec![("xyz:NVDA".to_string(), "NVDA".to_string())]
    );
    assert_eq!(keyword_mentions[0].matched_text, "graphics chips");
    assert_eq!(keyword_mentions[0].source, SymbolAliasSource::Keyword);
    assert_eq!(keyword_mentions[0].confidence, 74);
}

#[test]
fn ticker_source_beats_metadata_alias_for_same_symbol() {
    let symbols = vec![symbol_with_metadata(
        "xyz:NVDA",
        "NVDA",
        Some("Nvidia"),
        &["graphics chips"],
    )];

    let mentions = resolve_symbol_mentions("nvidia headlines before NVDA moved", &symbols);

    assert_eq!(mentions.len(), 1);
    assert_eq!(mentions[0].symbol_key, "xyz:NVDA");
    assert_eq!(mentions[0].matched_text, "NVDA");
    assert_eq!(mentions[0].source, SymbolAliasSource::Ticker);
}

#[test]
fn metadata_aliases_skip_unsafe_short_phrases() {
    let symbols = vec![symbol_with_metadata("xyz:NVDA", "NVDA", None, &["ai"])];

    let mentions = resolve_symbol_mentions("ai demand rose", &symbols);

    assert!(mentions.is_empty());
}

#[test]
fn generic_outcome_keywords_do_not_alias_every_outcome_market() {
    let mut outcome = symbol("#950", "OUT95-YES", MarketType::Outcome);
    outcome.keywords = vec![
        "outcome".to_string(),
        "prediction".to_string(),
        "will btc close green?".to_string(),
    ];
    outcome.outcome = Some(OutcomeSymbolInfo {
        outcome_id: 95,
        question_id: None,
        question_name: Some("Will BTC close green?".to_string()),
        question_description: None,
        question_class: None,
        question_underlying: None,
        question_expiry: None,
        question_price_thresholds: Vec::new(),
        question_period: None,
        question_named_outcomes: Vec::new(),
        question_settled_named_outcomes: Vec::new(),
        question_fallback_outcome: None,
        bucket_index: None,
        is_question_fallback: false,
        side_index: 0,
        side_name: "Yes".to_string(),
        outcome_name: "Recurring".to_string(),
        description: "Will BTC close green?".to_string(),
        class: None,
        underlying: None,
        expiry: None,
        target_price: None,
        period: None,
        quote_symbol: "USDH".to_string(),
        quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
        encoding: 950,
    });
    let symbols = vec![outcome];

    let generic = resolve_symbol_mentions("a bold prediction about the outcome", &symbols);
    assert!(generic.is_empty(), "{generic:?}");

    let question = resolve_symbol_mentions("will btc close green? odds shifted", &symbols);
    assert_eq!(
        pairs(&question),
        vec![("#950".to_string(), "OUT95-YES".to_string())]
    );
    assert_eq!(question[0].source, SymbolAliasSource::Keyword);
}

#[test]
fn key_suffix_aliases_include_stripped_leading_u() {
    let symbols = vec![symbol("xyz:UNATGAS", "UNATGAS", MarketType::Perp)];

    let mentions = resolve_symbol_mentions("NATGAS supply update", &symbols);

    assert_eq!(
        pairs(&mentions),
        vec![("xyz:UNATGAS".to_string(), "UNATGAS".to_string())]
    );
    assert_eq!(mentions[0].source, SymbolAliasSource::KeySuffix);
}
