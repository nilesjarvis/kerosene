use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};

use super::*;

mod matching;
mod ordering;

fn symbol(
    key: &str,
    ticker: &str,
    category: &str,
    display_name: Option<&str>,
    keywords: &[&str],
) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: category.to_string(),
        display_name: display_name.map(str::to_string),
        keywords: keywords.iter().map(|keyword| keyword.to_string()).collect(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn fallback_outcome_symbol() -> ExchangeSymbol {
    let mut symbol = symbol(
        "#660",
        "OUT66-YES",
        "outcome",
        Some("YES: fallback / other settlement"),
        &["outcome", "prediction", "fallback"],
    );
    symbol.market_type = MarketType::Outcome;
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
        is_question_fallback: true,
        side_index: 0,
        side_name: "Yes".to_string(),
        outcome_name: "Recurring Fallback".to_string(),
        description: "other".to_string(),
        class: None,
        underlying: None,
        expiry: None,
        target_price: None,
        period: None,
        quote_symbol: "USDH".to_string(),
        quote_token_index: Some(150),
        encoding: 660,
    });
    symbol
}
