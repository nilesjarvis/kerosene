use super::{LIVE_MID_MAX_AGE_MS, resolve_live_mid_from_candidates, valid_mid_price};
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::signing::OrderKind;
use std::collections::HashMap;

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

#[test]
fn valid_mid_price_accepts_only_positive_finite_values() {
    assert!(valid_mid_price(1.0));
    assert!(!valid_mid_price(0.0));
    assert!(!valid_mid_price(-1.0));
    assert!(!valid_mid_price(f64::NAN));
    assert!(!valid_mid_price(f64::INFINITY));
}

#[test]
fn live_mid_resolution_accepts_fresh_positive_finite_candidates() {
    let candidates = vec!["BTC".to_string()];
    let all_mids = HashMap::from([("BTC".to_string(), 100.0)]);
    let updated_at = HashMap::from([("BTC".to_string(), 10_000)]);

    assert_eq!(
        resolve_live_mid_from_candidates(&candidates, &all_mids, &updated_at, 10_001),
        Some(100.0)
    );
}

#[test]
fn live_mid_resolution_rejects_stale_missing_or_future_timestamps() {
    let now_ms = LIVE_MID_MAX_AGE_MS + 10_000;
    let candidates = vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()];
    let all_mids = HashMap::from([
        ("BTC".to_string(), 100.0),
        ("ETH".to_string(), 200.0),
        ("SOL".to_string(), 300.0),
    ]);
    let updated_at = HashMap::from([
        ("BTC".to_string(), now_ms - LIVE_MID_MAX_AGE_MS - 1),
        ("SOL".to_string(), now_ms + 1),
    ]);

    assert_eq!(
        resolve_live_mid_from_candidates(&candidates, &all_mids, &updated_at, now_ms),
        None
    );
}

#[test]
fn live_mid_resolution_uses_later_candidate_when_first_is_stale() {
    let now_ms = LIVE_MID_MAX_AGE_MS + 10_000;
    let candidates = vec!["BTC".to_string(), "UBTC".to_string()];
    let all_mids = HashMap::from([("BTC".to_string(), 100.0), ("UBTC".to_string(), 101.0)]);
    let updated_at = HashMap::from([
        ("BTC".to_string(), now_ms - LIVE_MID_MAX_AGE_MS - 1),
        ("UBTC".to_string(), now_ms),
    ]);

    assert_eq!(
        resolve_live_mid_from_candidates(&candidates, &all_mids, &updated_at, now_ms),
        Some(101.0)
    );
}

#[test]
fn refresh_order_price_for_symbol_seeds_limit_ioc_price_from_mid() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_kind = OrderKind::LimitIoc;
    terminal.order_price = "1".to_string();
    terminal.all_mids.insert("BTC".to_string(), 101.25);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    terminal.refresh_order_price_for_symbol("BTC");

    assert_eq!(terminal.order_price, format_price(101.25));
}

#[test]
fn restored_active_symbol_key_replaces_non_tradable_fallback_outcome() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        outcome_symbol("#660", true),
        outcome_symbol("#670", false),
        symbol("HYPE", MarketType::Perp),
    ];

    assert_eq!(
        terminal.restored_active_symbol_key("#660"),
        Some("HYPE".to_string())
    );
}

#[test]
fn validate_exchange_symbol_orderable_rejects_fallback_outcome() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![outcome_symbol("#660", true)];
    let symbol = terminal.exchange_symbols.first().expect("symbol");

    let error = terminal
        .validate_exchange_symbol_orderable(symbol, "Active")
        .expect_err("fallback outcomes are not orderable");

    assert!(error.contains("not a tradable market"));
}

#[test]
fn validate_exchange_symbol_orderable_rejects_outcome_without_metadata() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("#650", MarketType::Outcome)];
    let symbol = terminal.exchange_symbols.first().expect("symbol");

    let error = terminal
        .validate_exchange_symbol_orderable(symbol, "Active")
        .expect_err("outcome metadata is required before trading");

    assert!(error.contains("metadata is incomplete"));
}
