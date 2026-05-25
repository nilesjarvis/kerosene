use super::*;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};

fn outcome_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        key: "#1010".to_string(),
        ticker: "OUT101-YES".to_string(),
        category: "outcome".to_string(),
        display_name: Some("YES: Below 4.3%".to_string()),
        keywords: vec![
            "outcome".to_string(),
            "prediction".to_string(),
            "usdc".to_string(),
            "may cpi year-over-year".to_string(),
        ],
        asset_index: 100_001_010,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 101,
            question_id: Some(19),
            question_name: Some("May CPI year-over-year".to_string()),
            question_description: Some("Consumer Price Index release for May 2026".to_string()),
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: vec![101, 102, 103],
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: Some(100),
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Below 4.3%".to_string(),
            description: "This outcome resolves to Yes if CPI is below 4.3%.".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDC".to_string(),
            quote_token_index: Some(crate::api::USDC_TOKEN_INDEX),
            encoding: 1010,
        }),
    }
}

#[test]
fn outcome_search_matches_key_ticker_question_and_named_outcome() {
    let symbol = outcome_symbol();

    assert!(outcome_symbol_matches_search(&symbol, ""));
    assert!(outcome_symbol_matches_search(&symbol, "#1010"));
    assert!(outcome_symbol_matches_search(&symbol, "out101"));
    assert!(outcome_symbol_matches_search(&symbol, "may cpi"));
    assert!(outcome_symbol_matches_search(&symbol, "below 4.3"));
    assert!(outcome_symbol_matches_search(&symbol, "usdc"));
}

#[test]
fn outcome_search_requires_each_search_term_to_match() {
    let symbol = outcome_symbol();

    assert!(outcome_symbol_matches_search(&symbol, "cpi below"));
    assert!(!outcome_symbol_matches_search(&symbol, "cpi btc"));
}
