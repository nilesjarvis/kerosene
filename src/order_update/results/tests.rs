use super::*;
use crate::api::{ExchangeSymbol, MarketType, OrderStatusResult, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
use crate::order_execution::{OneShotPlacementContext, OrderSurface, PendingNukeExecution};
use crate::signing::{ExchangeOrderKind, ExchangeResponse};
use crate::timeframe::Timeframe;

mod cancel;
mod classification;
mod completion;
mod nuke;
mod one_shot;
mod pending;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

fn exchange_response(statuses: Vec<serde_json::Value>) -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": statuses
            }
        }
    }))
    .expect("test exchange response should deserialize")
}

fn cancel_exchange_response(statuses: Vec<serde_json::Value>) -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "cancel",
            "data": {
                "statuses": statuses
            }
        }
    }))
    .expect("test cancel exchange response should deserialize")
}

fn malformed_ok_response() -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": "schema-shifted"
            }
        }
    }))
    .expect("test exchange response should deserialize")
}

fn one_shot_context() -> OneShotPlacementContext {
    one_shot_context_with_kind(ExchangeOrderKind::Limit)
}

fn one_shot_context_with_kind(order_kind: ExchangeOrderKind) -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: TEST_ACCOUNT.to_string(),
        cloid: "0x00000000000000000000000000000000".to_string(),
        surface: OrderSurface::Ticket,
        symbol_key: "BTC".to_string(),
        order_kind,
    }
}

fn one_shot_context_with_cloid(
    cloid: &str,
    order_kind: ExchangeOrderKind,
) -> OneShotPlacementContext {
    OneShotPlacementContext {
        cloid: cloid.to_string(),
        ..one_shot_context_with_kind(order_kind)
    }
}

fn outcome_exchange_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT66-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 100_000_000,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        growth_mode: false,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 66,
            contract: crate::api::OutcomeContract::verified_fixture(),
            venue: None,
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
            bucket_index: Some(0),
            is_question_fallback: false,
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
            encoding: 660,
        }),
    }
}

fn one_shot_outcome_context(symbol_key: &str) -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: TEST_ACCOUNT.to_string(),
        cloid: "0x00000000000000000000000000000000".to_string(),
        surface: OrderSurface::Ticket,
        symbol_key: symbol_key.to_string(),
        order_kind: ExchangeOrderKind::Limit,
    }
}

fn nuke_context(symbol_key: &str) -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: TEST_ACCOUNT.to_string(),
        cloid: format!("0x{symbol_key:0<32}"),
        surface: OrderSurface::Nuke,
        symbol_key: symbol_key.to_string(),
        order_kind: ExchangeOrderKind::Market,
    }
}

fn terminal_with_connected_account() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
    terminal
}

fn order_status(status: &str) -> OrderStatusResult {
    OrderStatusResult {
        status: status.to_string(),
        oid: Some(42),
        cloid: Some("0x00000000000000000000000000000000".to_string()),
        raw_summary: format!("{status} (oid 42)"),
    }
}

fn begin_one_shot_status_request(
    terminal: &mut TradingTerminal,
    context: &OneShotPlacementContext,
) -> u64 {
    terminal.begin_one_shot_status_request(context)
}

fn finish_current_account_refresh(terminal: &mut TradingTerminal) {
    let context = terminal.current_account_data_request_context();
    let _task = terminal.apply_account_data_loaded(
        TEST_ACCOUNT.to_string(),
        context,
        Ok(account_data_with_open_orders(Vec::new())),
    );
}

fn open_order(oid: u64) -> crate::account::OpenOrder {
    open_order_for(oid, "BTC")
}

fn open_order_for(oid: u64, coin: &str) -> crate::account::OpenOrder {
    crate::account::OpenOrder {
        coin: coin.to_string(),
        side: "B".to_string(),
        limit_px: "100".to_string(),
        sz: "1".to_string(),
        oid,
        timestamp: 1,
        reduce_only: Some(false),
        is_trigger: None,
        order_type: None,
        tif: None,
        trigger_px: None,
    }
}

fn account_data_with_open_orders(
    orders: Vec<crate::account::OpenOrder>,
) -> crate::account::AccountData {
    crate::account::AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: crate::account::ClearinghouseState {
            margin_summary: crate::account::MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: crate::account::SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: orders,
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: crate::account::UserFeeRates::default(),
        completeness: crate::account::AccountDataCompleteness::default(),
        fetched_at_ms: 1,
    }
}

fn arm_pending_cancel_status_request(
    terminal: &mut TradingTerminal,
    account_address: &str,
    oid: u64,
    symbol: &str,
) {
    terminal.pending_cancel_status_request = Some(PendingCancelStatusRequest::new(
        account_address.to_string(),
        oid,
        symbol.to_string(),
    ));
}

fn terminal_with_pending_cancel() -> (TradingTerminal, Option<u64>) {
    let mut terminal = terminal_with_connected_account();
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    let order = open_order(42);
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_open_orders(vec![order.clone()]),
    );
    let pending_id =
        terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &order);
    arm_pending_cancel_status_request(&mut terminal, TEST_ACCOUNT, order.oid, &order.coin);
    assert!(pending_id.is_some());
    (terminal, pending_id)
}
