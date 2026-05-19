use super::*;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::signing::OrderKind;

fn outcome_info(is_question_fallback: bool) -> OutcomeSymbolInfo {
    OutcomeSymbolInfo {
        outcome_id: 65,
        question_id: Some(12),
        question_name: Some("Recurring".to_string()),
        question_description: Some(
            "class:priceBucket|underlying:BTC|expiry:20260520-0600".to_string(),
        ),
        question_class: Some("priceBucket".to_string()),
        question_underlying: Some("BTC".to_string()),
        question_expiry: Some("20260520-0600".to_string()),
        question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
        question_period: Some("1d".to_string()),
        question_named_outcomes: vec![67, 68, 69],
        question_settled_named_outcomes: Vec::new(),
        question_fallback_outcome: Some(66),
        bucket_index: Some(0),
        is_question_fallback,
        side_index: 0,
        side_name: "Yes".to_string(),
        outcome_name: "Recurring Named Outcome".to_string(),
        description: "index:0".to_string(),
        class: None,
        underlying: None,
        expiry: None,
        target_price: None,
        period: None,
        quote_symbol: "USDH".to_string(),
        quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
        encoding: 650,
    }
}

fn outcome_symbol(key: &str, asset_index: u32, is_question_fallback: bool) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT65-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(outcome_info(is_question_fallback)),
    }
}

fn terminal_for_outcome_order(symbol: ExchangeSymbol) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = sensitive_string("agent-key");
    terminal.active_symbol = symbol.key.clone();
    terminal.exchange_symbols = vec![symbol];
    terminal.order_kind = OrderKind::Limit;
    terminal.order_price = "0.42123456".to_string();
    terminal.order_quantity = "3".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.order_reduce_only = true;
    terminal
        .all_mids
        .insert(terminal.active_symbol.clone(), 0.42);
    terminal
        .all_mids_updated_at_ms
        .insert(terminal.active_symbol.clone(), TradingTerminal::now_ms());
    terminal
}

#[test]
fn outcome_order_preparation_builds_spot_like_probability_payload() {
    let terminal = terminal_for_outcome_order(outcome_symbol("#650", 100_000_650, false));
    let sym = terminal.exchange_symbols.first().expect("symbol");

    let prepared = terminal
        .prepare_order_submission(sym, true)
        .expect("valid outcome order");

    assert_eq!(
        prepared,
        PreparedOrderSubmission {
            asset: 100_000_650,
            is_buy: true,
            price: "0.42123".to_string(),
            size: "3".to_string(),
            order_kind: OrderKind::Limit,
            reduce_only: false,
            is_outcome: true,
        }
    );
}

#[test]
fn execute_order_rejects_non_tradable_fallback_outcome_before_signing() {
    let mut terminal = terminal_for_outcome_order(outcome_symbol("#660", 100_000_660, true));

    let _task = terminal.execute_order(true);

    let (message, is_error) = terminal.order_status.as_ref().expect("status");
    assert!(*is_error);
    assert!(message.contains("not a tradable market"));
    assert!(terminal.pending_order_action.is_none());
}
