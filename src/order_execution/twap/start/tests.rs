use super::price_range::parse_positive_price;
use super::validation::{
    TwapStartSchedule, parse_twap_start_schedule, validate_twap_schedule_capacity,
};
use super::*;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::sensitive_string;
use crate::config::AccountProfile;
use crate::signing::OrderKind;
use crate::twap_state::{TwapOrder, TwapStatus};

mod prices;
mod schedule;
mod submission;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn fallback_outcome_symbol(key: &str) -> ExchangeSymbol {
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
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 660,
        }),
        ..symbol(key, MarketType::Outcome)
    }
}

fn twap_ready_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.order_kind = OrderKind::Twap;
    terminal.order_quantity = "2.5".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.twap_form.duration_minutes = "5".to_string();
    terminal.twap_form.slices = "2".to_string();
    terminal.twap_form.min_price = "90".to_string();
    terminal.twap_form.max_price = "110".to_string();
    terminal.twap_form.randomize = false;
    terminal.pending_order_action = None;
    terminal
}

fn started_twap_or_panic(terminal: &TradingTerminal) -> &TwapOrder {
    let Some(twap_id) = terminal.selected_twap_id else {
        panic!("started twap should be selected");
    };
    match terminal.twap_orders.get(&twap_id) {
        Some(twap) => twap,
        None => panic!("twap order should be inserted"),
    }
}

fn parse_schedule_or_panic(duration_minutes: &str, slices: &str) -> TwapStartSchedule {
    match parse_twap_start_schedule(duration_minutes, slices) {
        Ok(schedule) => schedule,
        Err(error) => panic!("valid TWAP schedule: {error}"),
    }
}

fn schedule_error_or_panic(duration_minutes: &str, slices: &str) -> String {
    match parse_twap_start_schedule(duration_minutes, slices) {
        Ok(_) => panic!("invalid TWAP schedule should fail"),
        Err(error) => error,
    }
}

fn schedule_capacity_error_or_panic(active_slice_rate: f64, schedule: TwapStartSchedule) -> String {
    match validate_twap_schedule_capacity(
        active_slice_rate,
        schedule.duration,
        schedule.slice_count,
    ) {
        Ok(()) => panic!("dense TWAP schedule should fail"),
        Err(error) => error,
    }
}

fn order_status_error_contains(terminal: &TradingTerminal, needle: &str) -> bool {
    terminal
        .order_status
        .as_ref()
        .is_some_and(|(message, is_error)| *is_error && message.contains(needle))
}
