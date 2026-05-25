use super::*;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;

fn outcome_symbol() -> ExchangeSymbol {
    outcome_symbol_with(101, 0, Some(19), "Below 4.3%")
}

fn outcome_symbol_with(
    outcome_id: u32,
    side_index: u32,
    question_id: Option<u32>,
    name: &str,
) -> ExchangeSymbol {
    let side_name = if side_index == 0 { "Yes" } else { "No" };
    ExchangeSymbol {
        key: format!("#{}", outcome_id * 10 + side_index),
        ticker: format!("OUT{}-{}", outcome_id, side_name.to_ascii_uppercase()),
        category: "outcome".to_string(),
        display_name: Some(format!("{}: {name}", side_name.to_ascii_uppercase())),
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
            outcome_id,
            question_id,
            question_name: question_id.map(|_| "May CPI year-over-year".to_string()),
            question_description: question_id
                .map(|_| "Consumer Price Index release for May 2026".to_string()),
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: question_id.map(|_| vec![101, 102, 103]).unwrap_or_default(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: question_id.map(|_| 100),
            bucket_index: None,
            is_question_fallback: false,
            side_index,
            side_name: side_name.to_string(),
            outcome_name: name.to_string(),
            description: format!("This outcome resolves to Yes if CPI is {name}."),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDC".to_string(),
            quote_token_index: Some(crate::api::USDC_TOKEN_INDEX),
            encoding: outcome_id * 10 + side_index,
        }),
    }
}

#[test]
fn outcome_market_groups_use_question_relationships() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        outcome_symbol_with(101, 0, Some(19), "Below 4.3%"),
        outcome_symbol_with(101, 1, Some(19), "Below 4.3%"),
        outcome_symbol_with(102, 0, Some(19), "Exactly 4.3%"),
        outcome_symbol_with(102, 1, Some(19), "Exactly 4.3%"),
        outcome_symbol_with(95, 0, None, "BTC above 77,363"),
        outcome_symbol_with(95, 1, None, "BTC above 77,363"),
    ];

    let groups = terminal.grouped_outcome_markets();
    let question_group = groups
        .iter()
        .find(|group| group.key == "question:19")
        .expect("question group should be present");
    let standalone_group = groups
        .iter()
        .find(|group| group.key == "outcome:95")
        .expect("standalone outcome group should be present");

    assert_eq!(groups.len(), 2);
    assert_eq!(question_group.title, "May CPI year-over-year");
    assert!(question_group.is_question_group);
    assert_eq!(question_group.outcome_count, 2);
    assert_eq!(question_group.trade_coin_count, 4);
    assert!(question_group.outcomes.contains_key(&101));
    assert!(question_group.outcomes.contains_key(&102));
    assert!(!standalone_group.is_question_group);
    assert_eq!(standalone_group.outcome_count, 1);
    assert_eq!(standalone_group.trade_coin_count, 2);
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
