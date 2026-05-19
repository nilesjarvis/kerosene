use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::market_state::SymbolSearchMarketFilter;

use super::*;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.rsplit(':').next().unwrap_or(key).to_string(),
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

fn outcome_symbol(key: &str, is_question_fallback: bool) -> ExchangeSymbol {
    let mut symbol = symbol(key, MarketType::Outcome);
    symbol.outcome = Some(OutcomeSymbolInfo {
        outcome_id: 66,
        question_id: Some(12),
        question_name: Some("Recurring".to_string()),
        question_description: Some(
            "class:priceBucket|underlying:BTC|expiry:20260520-0600|priceThresholds:75348,78423|period:1d"
                .to_string(),
        ),
        question_class: Some("priceBucket".to_string()),
        question_underlying: Some("BTC".to_string()),
        question_expiry: Some("20260520-0600".to_string()),
        question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
        question_period: Some("1d".to_string()),
        question_named_outcomes: vec![67, 68, 69],
        question_settled_named_outcomes: Vec::new(),
        question_fallback_outcome: Some(66),
        bucket_index: None,
        is_question_fallback,
        side_index: 0,
        side_name: "Yes".to_string(),
        outcome_name: if is_question_fallback {
            "Recurring Fallback".to_string()
        } else {
            "Recurring Named Outcome".to_string()
        },
        description: if is_question_fallback {
            "other".to_string()
        } else {
            "index:0".to_string()
        },
        class: None,
        underlying: None,
        expiry: None,
        target_price: None,
        period: None,
        quote_symbol: "USDH".to_string(),
        quote_token_index: Some(150),
        encoding: if is_question_fallback { 660 } else { 670 },
    });
    symbol
}

#[test]
fn hip3_dexes_are_sorted_unique_perp_prefixes_only() {
    let symbols = vec![
        symbol("xyz:NVDA", MarketType::Perp),
        symbol("@1", MarketType::Spot),
        symbol("abc:BTC", MarketType::Perp),
        symbol("xyz:TSLA", MarketType::Perp),
        symbol("BTC", MarketType::Perp),
    ];

    assert_eq!(
        symbol_search_hip3_dexes(&symbols),
        vec!["abc".to_string(), "xyz".to_string()]
    );
}

#[test]
fn market_filter_matches_native_spot_outcome_and_hip3_variants() {
    let native = symbol("BTC", MarketType::Perp);
    let hip3 = symbol("xyz:NVDA", MarketType::Perp);
    let spot = symbol("@1", MarketType::Spot);
    let outcome = symbol("#0", MarketType::Outcome);

    assert!(symbol_search_matches_market_filter(
        &native,
        SymbolSearchMarketFilter::NativePerps,
        None
    ));
    assert!(!symbol_search_matches_market_filter(
        &hip3,
        SymbolSearchMarketFilter::NativePerps,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &spot,
        SymbolSearchMarketFilter::Spot,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &outcome,
        SymbolSearchMarketFilter::Outcomes,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &hip3,
        SymbolSearchMarketFilter::Hip3,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &hip3,
        SymbolSearchMarketFilter::Hip3,
        Some("xyz")
    ));
    assert!(!symbol_search_matches_market_filter(
        &hip3,
        SymbolSearchMarketFilter::Hip3,
        Some("abc")
    ));
}

#[test]
fn market_filter_hides_question_fallback_outcomes() {
    let fallback = outcome_symbol("#660", true);
    let named = outcome_symbol("#670", false);

    assert!(!symbol_search_matches_market_filter(
        &fallback,
        SymbolSearchMarketFilter::All,
        None
    ));
    assert!(!symbol_search_matches_market_filter(
        &fallback,
        SymbolSearchMarketFilter::Outcomes,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &named,
        SymbolSearchMarketFilter::All,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &named,
        SymbolSearchMarketFilter::Outcomes,
        None
    ));
}

#[test]
fn exchange_labels_match_market_kind() {
    assert_eq!(
        symbol_search_exchange_label(&symbol("BTC", MarketType::Perp)),
        "Native Perps"
    );
    assert_eq!(
        symbol_search_exchange_label(&symbol("xyz:NVDA", MarketType::Perp)),
        "HIP-3: xyz"
    );
    assert_eq!(
        symbol_search_exchange_label(&symbol("@1", MarketType::Spot)),
        "Spot"
    );
    assert_eq!(
        symbol_search_exchange_label(&symbol("#0", MarketType::Outcome)),
        "Outcomes"
    );
}

#[test]
fn exchange_rank_groups_native_spot_hip3_and_outcomes() {
    assert_eq!(
        symbol_search_exchange_rank(&symbol("BTC", MarketType::Perp)),
        (0, String::new())
    );
    assert_eq!(
        symbol_search_exchange_rank(&symbol("@1", MarketType::Spot)),
        (1, String::new())
    );
    assert_eq!(
        symbol_search_exchange_rank(&symbol("xyz:NVDA", MarketType::Perp)),
        (2, "xyz".to_string())
    );
    assert_eq!(
        symbol_search_exchange_rank(&symbol("#0", MarketType::Outcome)),
        (3, String::new())
    );
}
