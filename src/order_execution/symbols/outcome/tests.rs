use super::{OUTCOME_MAX_PRICE, OUTCOME_MIN_PRICE};
use crate::api;
use crate::app_state::TradingTerminal;
use chrono::TimeZone;

mod labels;
mod pricing;
mod terminal_display;

fn outcome_info() -> api::OutcomeSymbolInfo {
    api::OutcomeSymbolInfo {
        outcome_id: 65,
        question_id: None,
        question_name: None,
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
        description: "class:priceBinary|underlying:BTC".to_string(),
        class: Some("priceBinary".to_string()),
        underlying: Some("BTC".to_string()),
        expiry: Some("20260520-0600".to_string()),
        target_price: Some("76886".to_string()),
        period: Some("1d".to_string()),
        quote_symbol: "USDH".to_string(),
        quote_token_index: Some(150),
        encoding: 650,
    }
}

fn spot_symbol(key: &str, ticker: &str, display: &str) -> api::ExchangeSymbol {
    api::ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(display.to_string()),
        keywords: Vec::new(),
        asset_index: 10107,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: api::MarketType::Spot,
        outcome: None,
    }
}

fn utc_ms(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> u64 {
    match chrono::Utc
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
    {
        Some(timestamp) => timestamp.timestamp_millis() as u64,
        None => panic!("invalid UTC timestamp {year}-{month}-{day} {hour}:{minute}"),
    }
}
