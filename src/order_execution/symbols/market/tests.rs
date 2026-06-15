use super::{LIVE_MID_MAX_AGE_MS, resolve_live_mid_from_candidates, valid_mid_price};
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::signing::OrderKind;
use std::collections::HashMap;

mod active_symbol;
mod live_mids;
mod mid_candidates;
mod orderability;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "test".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn outcome_symbol(key: &str, is_question_fallback: bool) -> ExchangeSymbol {
    ExchangeSymbol {
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 66,
            question_id: Some(12),
            question_name: Some("Recurring".to_string()),
            question_description: None,
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
            outcome_name: "Recurring Fallback".to_string(),
            description: "other".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 660,
        }),
        ..symbol(key, MarketType::Outcome)
    }
}

fn first_symbol(terminal: &TradingTerminal) -> &ExchangeSymbol {
    match terminal.exchange_symbols.first() {
        Some(symbol) => symbol,
        None => panic!("terminal should contain an exchange symbol"),
    }
}

fn orderability_error(terminal: &TradingTerminal, symbol: &ExchangeSymbol) -> String {
    match terminal.validate_exchange_symbol_orderable(symbol, "Active") {
        Ok(()) => panic!("symbol should not be orderable"),
        Err(error) => error,
    }
}
