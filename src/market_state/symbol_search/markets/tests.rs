use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::market_state::SymbolSearchMarketFilter;

use super::*;

mod filter;
mod hip3;
mod labels;

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
            concat!(
                "class:priceBucket|underlying:BTC|expiry:20260520-0600|",
                "priceThresholds:75348,78423|period:1d"
            )
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
