use super::*;
use crate::annotations::DrawingTool;
use crate::api::{ExchangeSymbol, MarketType, OrderStatusResult, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartInstance, ChartSurfaceId, DetachedChartWindowState};
use crate::message::Message;
use crate::order_execution::{
    OneShotPlacementContext, OrderSurface, PendingNukeExecution, QuickOrderForm,
};
use crate::signing::ExchangeOrderKind;
use crate::timeframe::Timeframe;

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

#[test]
fn pending_one_shot_status_request_debug_redacts_account_and_cloid() {
    let request = PendingOneShotStatusRequest::new(7, &one_shot_context());

    let rendered = format!("{request:?}");

    assert!(rendered.contains("account_address: \"<redacted>\""));
    assert!(rendered.contains("cloid: \"<redacted>\""));
    assert!(!rendered.contains(TEST_ACCOUNT));
    assert!(!rendered.contains("0x00000000000000000000000000000000"));
}

#[test]
fn pending_cancel_status_request_debug_redacts_account_and_oid() {
    let request = PendingCancelStatusRequest::new(TEST_ACCOUNT.to_string(), 42, "BTC".to_string());

    let rendered = format!("{request:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(TEST_ACCOUNT));
    assert!(!rendered.contains("42"));
    assert!(rendered.contains("BTC"));
}

#[test]
fn pending_move_status_request_debug_redacts_account_and_oid() {
    let request = PendingMoveStatusRequest::new(TEST_ACCOUNT.to_string(), 42, "BTC".to_string());

    let rendered = format!("{request:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(TEST_ACCOUNT));
    assert!(!rendered.contains("42"));
    assert!(rendered.contains("BTC"));
}

fn quick_order_form() -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: "1".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 300.0,
        chart_h: 200.0,
    }
}

#[test]
fn detached_window_escape_clears_only_matching_chart_transient_state() {
    let mut terminal = TradingTerminal::boot().0;
    let main_window_id = iced::window::Id::unique();
    let detached_window_id = iced::window::Id::unique();
    let other_window_id = iced::window::Id::unique();
    let detached_chart_id = 7;
    let other_chart_id = 8;
    let detached_surface = ChartSurfaceId::Detached(detached_window_id);
    let other_surface = ChartSurfaceId::Detached(other_window_id);
    terminal.main_window_id = Some(main_window_id);
    terminal.charts.clear();
    terminal.detached_chart_windows.clear();
    terminal.chart_quick_order_surface.clear();
    terminal.chart_surface_active_tools.clear();

    let mut detached_chart =
        ChartInstance::new(detached_chart_id, "BTC".to_string(), Timeframe::H1);
    detached_chart.chart.set_surface_id(detached_surface);
    detached_chart.editor_open = true;
    detached_chart.editor_search_query = "bt".to_string();
    detached_chart.editor_selected_index = Some(0);
    detached_chart.chart.active_tool = Some(DrawingTool::TrendLine);
    detached_chart.set_quick_order(quick_order_form());

    let mut other_chart = ChartInstance::new(other_chart_id, "ETH".to_string(), Timeframe::H1);
    other_chart.chart.set_surface_id(other_surface);
    other_chart.editor_open = true;
    other_chart.editor_search_query = "et".to_string();
    other_chart.editor_selected_index = Some(0);
    other_chart.chart.active_tool = Some(DrawingTool::HorizontalLevel);
    other_chart.set_quick_order(quick_order_form());

    terminal.charts.insert(detached_chart_id, detached_chart);
    terminal.charts.insert(other_chart_id, other_chart);
    terminal.detached_chart_windows.insert(
        detached_window_id,
        DetachedChartWindowState::new(detached_chart_id),
    );
    terminal.detached_chart_windows.insert(
        other_window_id,
        DetachedChartWindowState::new(other_chart_id),
    );
    terminal
        .chart_quick_order_surface
        .insert(detached_chart_id, detached_surface);
    terminal
        .chart_quick_order_surface
        .insert(other_chart_id, other_surface);
    terminal
        .chart_surface_active_tools
        .insert(detached_surface, DrawingTool::TrendLine);
    terminal
        .chart_surface_active_tools
        .insert(other_surface, DrawingTool::HorizontalLevel);

    let _ = terminal.update_order(Message::EscapePressed(detached_window_id));

    let detached_chart = terminal
        .charts
        .get(&detached_chart_id)
        .expect("detached chart");
    assert!(!detached_chart.editor_open);
    assert!(detached_chart.editor_search_query.is_empty());
    assert_eq!(detached_chart.editor_selected_index, None);
    assert!(detached_chart.quick_order.is_none());
    assert!(!detached_chart.chart.quick_order_open);
    assert_eq!(detached_chart.chart.active_tool, None);
    assert!(
        !terminal
            .chart_quick_order_surface
            .contains_key(&detached_chart_id)
    );
    assert!(
        !terminal
            .chart_surface_active_tools
            .contains_key(&detached_surface)
    );

    let other_chart = terminal.charts.get(&other_chart_id).expect("other chart");
    assert!(other_chart.editor_open);
    assert_eq!(other_chart.editor_search_query, "et");
    assert_eq!(other_chart.editor_selected_index, Some(0));
    assert!(other_chart.quick_order.is_some());
    assert!(other_chart.chart.quick_order_open);
    assert_eq!(
        other_chart.chart.active_tool,
        Some(DrawingTool::HorizontalLevel)
    );
    assert_eq!(
        terminal
            .chart_quick_order_surface
            .get(&other_chart_id)
            .copied(),
        Some(other_surface)
    );
    assert_eq!(
        terminal
            .chart_surface_active_tools
            .get(&other_surface)
            .copied(),
        Some(DrawingTool::HorizontalLevel)
    );
}

#[test]
fn successful_exchange_results_require_account_refresh() {
    let resting = exchange_response(vec![serde_json::json!({
        "resting": {
            "oid": 42_u64
        }
    })]);
    let filled = exchange_response(vec![serde_json::json!({
        "filled": {
            "totalSz": "1",
            "avgPx": "100",
            "oid": 43_u64
        }
    })]);
    let cancel = exchange_response(vec![serde_json::json!("success")]);

    assert!(result_requires_account_refresh(&Ok(resting)));
    assert!(result_requires_account_refresh(&Ok(filled)));
    assert!(result_requires_account_refresh(&Ok(cancel)));
}

#[test]
fn pure_exchange_error_responses_do_not_require_account_refresh() {
    let exchange_error = exchange_response(vec![serde_json::json!({
        "error": "Order rejected"
    })]);

    assert!(!result_requires_account_refresh(&Ok(exchange_error)));
}

#[test]
fn mixed_exchange_error_responses_require_account_refresh() {
    let later_exchange_error = exchange_response(vec![
        serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }),
        serde_json::json!({
            "error": "Second order rejected"
        }),
    ]);

    assert!(result_requires_account_refresh(&Ok(later_exchange_error)));
}

#[test]
fn ambiguous_transport_results_require_account_refresh() {
    assert!(result_requires_account_refresh(&Err(
        "Exchange request failed: connection closed before response".to_string()
    )));
    assert!(result_requires_account_refresh(&Err(
        "Failed to read response: request body timed out".to_string()
    )));
    assert!(result_requires_account_refresh(&Err(
        "Exchange error: not-json response".to_string()
    )));
}

#[test]
fn execution_result_classifier_normalizes_successful_outcomes() {
    let resting = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "resting": {
            "oid": 42_u64
        }
    })])));
    assert_eq!(resting.kind, ExecutionOutcomeKind::AcceptedResting);
    assert_eq!(resting.status, "Resting (oid 42)");
    assert!(!resting.is_error);
    assert!(resting.refresh_account);

    let filled = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "filled": {
            "totalSz": "1",
            "avgPx": "100",
            "oid": 43_u64
        }
    })])));
    assert_eq!(filled.kind, ExecutionOutcomeKind::Filled);
    assert!(!filled.is_error);
    assert!(filled.refresh_account);

    let filled_without_oid =
        classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100"
            }
        })])));
    assert_eq!(filled_without_oid.kind, ExecutionOutcomeKind::Ambiguous);
    assert_eq!(filled_without_oid.status, "Filled 1 @ $100 (oid ?)");
    assert!(filled_without_oid.is_error);
    assert!(filled_without_oid.refresh_account);

    let cancelled =
        classify_execution_result(Ok(cancel_exchange_response(vec![serde_json::json!(
            "success"
        )])));
    assert_eq!(cancelled.kind, ExecutionOutcomeKind::Cancelled);
    assert_eq!(cancelled.status, "Cancelled");
    assert!(cancelled.refresh_account);

    // A modify/order ack with the same bare-"success" status must not be
    // classified as a cancel; it falls through to Ambiguous, which routes
    // placements into the cloid status check instead of reporting a cancel.
    let acknowledged =
        classify_execution_result(Ok(exchange_response(vec![serde_json::json!("success")])));
    assert_ne!(acknowledged.kind, ExecutionOutcomeKind::Cancelled);
    assert_eq!(acknowledged.status, "Success");
}

#[test]
fn execution_result_classifier_separates_rejected_ambiguous_and_transport_unknown() {
    let rejected = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "error": "Order rejected"
    })])));
    assert_eq!(rejected.kind, ExecutionOutcomeKind::Rejected);
    assert!(rejected.is_error);
    assert!(!rejected.refresh_account);

    let mixed = classify_execution_result(Ok(exchange_response(vec![
        serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }),
        serde_json::json!({
            "error": "Second order rejected"
        }),
    ])));
    assert_eq!(mixed.kind, ExecutionOutcomeKind::Ambiguous);
    assert!(mixed.status.contains("Resting (oid 42)"));
    assert!(mixed.status.contains("Error: Second order rejected"));
    assert!(mixed.is_error);
    assert!(mixed.refresh_account);

    let unstructured_top_level_error: ExchangeResponse =
        serde_json::from_value(serde_json::json!({
            "status": "err",
            "response": "request rejected before execution"
        }))
        .expect("unstructured error response should deserialize");
    let top_level_rejected = classify_execution_result(Ok(unstructured_top_level_error));
    assert_eq!(top_level_rejected.kind, ExecutionOutcomeKind::Rejected);
    assert!(top_level_rejected.is_error);
    assert!(!top_level_rejected.refresh_account);

    let top_level_error_with_effect: ExchangeResponse = serde_json::from_value(serde_json::json!({
        "status": "err",
        "response": {
            "type": "order",
            "data": {
                "statuses": [{
                    "resting": {
                        "oid": 42_u64
                    }
                }]
            }
        }
    }))
    .expect("contradictory response should deserialize");
    let top_level_mixed = classify_execution_result(Ok(top_level_error_with_effect));
    assert_eq!(top_level_mixed.kind, ExecutionOutcomeKind::Ambiguous);
    assert!(top_level_mixed.is_error);
    assert!(top_level_mixed.refresh_account);

    let ambiguous = classify_execution_result(Ok(malformed_ok_response()));
    assert_eq!(ambiguous.kind, ExecutionOutcomeKind::Ambiguous);
    assert_eq!(ambiguous.status, "No response body");
    assert!(ambiguous.is_error);
    assert!(ambiguous.refresh_account);

    let unknown = classify_execution_result(Err(
        "Exchange request failed: connection closed before response".to_string(),
    ));
    assert_eq!(unknown.kind, ExecutionOutcomeKind::TransportUnknown);
    assert!(unknown.is_error);
    assert!(unknown.refresh_account);
}

#[test]
fn execution_result_classifier_redacts_sensitive_external_errors() {
    let rejected = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "error": "Order rejected private_key=super-secret"
    })])));
    assert_eq!(rejected.kind, ExecutionOutcomeKind::Rejected);
    assert!(rejected.status.contains("Order rejected"));
    assert!(rejected.status.contains("private_key=<redacted>"));
    assert!(!rejected.status.contains("super-secret"));

    let unknown = classify_execution_result(Err(
        "Exchange request failed: token=transport-secret".to_string()
    ));
    assert_eq!(unknown.kind, ExecutionOutcomeKind::TransportUnknown);
    assert!(unknown.status.contains("Exchange request failed"));
    assert!(unknown.status.contains("token=<redacted>"));
    assert!(!unknown.status.contains("transport-secret"));
}

#[test]
fn one_shot_ambiguous_outcome_sets_cloid_reconciliation_status() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal.apply_one_shot_placement_outcome(
        one_shot_context(),
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::TransportUnknown,
            status: "exchange request failed".to_string(),
            is_error: true,
            refresh_account: true,
        },
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket placement status unknown for BTC"));
    assert!(message.contains("exchange request failed"));
    assert!(message.contains("checking 0x00000000000000000000000000000000"));
}

#[test]
fn one_shot_mixed_exchange_error_starts_cloid_reconciliation() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal.handle_order_result(
        None,
        one_shot_context(),
        Ok(exchange_response(vec![
            serde_json::json!({
                "resting": {
                    "oid": 42_u64
                }
            }),
            serde_json::json!({
                "error": "Second order rejected"
            }),
        ])),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket placement status unknown for BTC"));
    assert!(message.contains("Resting (oid 42)"));
    assert!(message.contains("Error: Second order rejected"));
}

#[test]
fn one_shot_order_status_result_normalizes_terminal_statuses() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("open")),
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert!(message.contains("Ticket placement confirmed by orderStatus for BTC"));
    assert!(terminal.pending_one_shot_status_requests.is_empty());

    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);
    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("rejected")),
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket placement rejected according to orderStatus for BTC"));
    assert!(terminal.pending_one_shot_status_requests.is_empty());
}

#[test]
fn one_shot_missing_status_stays_pending_until_account_refresh() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("unknownOid")),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("placement status still uncertain"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_one_shot_status_requests.is_empty());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn one_shot_canceled_status_stays_pending_until_account_refresh() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("canceled")),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("placement status still uncertain"));
    assert!(message.contains("refreshing account data"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_one_shot_status_requests.is_empty());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn one_shot_status_error_stays_pending_until_account_refresh() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Err("orderStatus request failed: private_key=super-secret".to_string()),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("placement status still uncertain"));
    assert!(message.contains("orderStatus request failed"));
    assert!(message.contains("private_key=<redacted>"));
    assert!(!message.contains("super-secret"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_one_shot_status_requests.is_empty());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn incomplete_account_refresh_does_not_clear_one_shot_status_request() {
    for incomplete_section in [
        crate::account::AccountDataSection::OpenOrders,
        crate::account::AccountDataSection::Fills,
    ] {
        let mut terminal = terminal_with_connected_account();
        let context = one_shot_context();
        let request_id = begin_one_shot_status_request(&mut terminal, &context);

        let _task = terminal.handle_one_shot_placement_status_result(
            request_id,
            context,
            Err("orderStatus temporarily unavailable".to_string()),
        );

        let mut data = account_data_with_open_orders(Vec::new());
        data.completeness
            .mark_incomplete(incomplete_section, "test section unavailable");
        let refresh_context = terminal.current_account_data_request_context();
        let _task =
            terminal.apply_account_data_loaded(TEST_ACCOUNT.to_string(), refresh_context, Ok(data));

        assert!(
            terminal.has_pending_one_shot_status_requests_for_test(),
            "{incomplete_section:?} must not resolve an uncertain placement"
        );
        assert!(terminal.has_pending_trading_request());
    }
}

#[test]
fn account_refresh_must_cover_one_shot_symbol_before_clearing_status_request() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_outcome_context("flx:BTC");
    let request_id = begin_one_shot_status_request(&mut terminal, &context);
    terminal.market_universe = crate::config::MarketUniverseConfig::hip3_dex("xyz");

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Err("orderStatus temporarily unavailable".to_string()),
    );

    let mut unrelated_scope = account_data_with_open_orders(Vec::new());
    unrelated_scope.fetch_scope = crate::account::AccountDataFetchScope::hip3_dex("xyz");
    let refresh_context = terminal.current_account_data_request_context();
    let _task = terminal.apply_account_data_loaded(
        TEST_ACCOUNT.to_string(),
        refresh_context,
        Ok(unrelated_scope),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.has_pending_trading_request());

    terminal.market_universe = crate::config::MarketUniverseConfig::hip3_dex("flx");
    let _task = terminal.refresh_account_data();
    let mut covering_scope = account_data_with_open_orders(Vec::new());
    covering_scope.fetch_scope = crate::account::AccountDataFetchScope::hip3_dex("flx");
    let refresh_context = terminal.current_account_data_request_context();
    let _task = terminal.apply_account_data_loaded(
        TEST_ACCOUNT.to_string(),
        refresh_context,
        Ok(covering_scope),
    );

    assert!(terminal.pending_one_shot_status_requests.is_empty());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn one_shot_status_result_with_stale_request_id_is_ignored() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id.wrapping_add(1),
        context,
        Ok(order_status("open")),
    );

    assert!(terminal.order_status.is_none());
    assert_eq!(
        terminal
            .pending_one_shot_status_requests
            .values()
            .next()
            .map(|request| request.request_id),
        Some(request_id)
    );
}

#[test]
fn older_status_request_survives_newer_unrelated_outcome() {
    // Regression: with one shared slot, a second placement outcome used to
    // clobber the first placement's ambiguity check and its orderStatus
    // response was silently dropped. Each cloid must resolve independently.
    let mut terminal = terminal_with_connected_account();
    let old_context = one_shot_context_with_kind(ExchangeOrderKind::Market);
    let _task = terminal.apply_one_shot_placement_outcome(
        old_context.clone(),
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::TransportUnknown,
            status: "exchange request failed".to_string(),
            is_error: true,
            refresh_account: false,
        },
    );
    let old_request_id = terminal
        .pending_one_shot_status_requests
        .values()
        .next()
        .expect("status request should be pending")
        .request_id;

    let newer_context = one_shot_context_with_cloid(
        "0x00000000000000000000000000000001",
        ExchangeOrderKind::Limit,
    );
    let _task = terminal.apply_one_shot_placement_outcome(
        newer_context,
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::Filled,
            status: "Filled avgPx=100 totalSz=1".to_string(),
            is_error: false,
            refresh_account: false,
        },
    );
    assert!(
        terminal.has_pending_one_shot_status_requests_for_test(),
        "unrelated outcome must not clear the older ambiguity check"
    );

    let _task = terminal.handle_one_shot_placement_status_result(
        old_request_id,
        old_context,
        Ok(order_status("open")),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("unexpectedly rested"));
    assert!(terminal.pending_one_shot_status_requests.is_empty());
}

#[test]
fn one_shot_ioc_like_order_status_open_is_unexpected_resting_error() {
    for order_kind in [ExchangeOrderKind::Market, ExchangeOrderKind::LimitIoc] {
        let mut terminal = terminal_with_connected_account();
        let context = one_shot_context_with_kind(order_kind);
        let request_id = begin_one_shot_status_request(&mut terminal, &context);

        let _task = terminal.handle_one_shot_placement_status_result(
            request_id,
            context,
            Ok(order_status("open")),
        );

        let (message, is_error) = terminal.order_status.clone().expect("status should be set");
        assert!(is_error);
        assert!(message.contains("Ticket"));
        assert!(message.contains(order_kind.label()));
        assert!(message.contains("unexpectedly rested"));
        assert!(message.contains("cancel 0x00000000000000000000000000000000"));
    }
}

#[test]
fn one_shot_ioc_like_direct_resting_response_is_unexpected_resting_error() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal.handle_order_result(
        None,
        one_shot_context_with_kind(ExchangeOrderKind::Market),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket market order unexpectedly rested for BTC"));
    assert!(message.contains("Resting (oid 42)"));
}

#[test]
fn one_shot_limit_direct_resting_response_remains_successful() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal.handle_order_result(
        None,
        one_shot_context_with_kind(ExchangeOrderKind::Limit),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "Resting (oid 42)");
}

#[test]
fn one_shot_success_during_refresh_backoff_marks_reconciliation_required() {
    let mut terminal = terminal_with_connected_account();
    terminal.account_refresh_backoff_until_ms = Some(TradingTerminal::now_ms() + 60_000);

    let _task = terminal.handle_order_result(
        None,
        one_shot_context_with_kind(ExchangeOrderKind::Limit),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    assert!(!terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    assert!(
        terminal
            .account_error
            .as_deref()
            .is_some_and(|error| error.contains("rate limited"))
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "Resting (oid 42)");
}

#[test]
fn one_shot_statuses_use_outcome_display_label_not_raw_key() {
    let mut terminal = terminal_with_connected_account();
    let sym = outcome_exchange_symbol("#660");
    let label = TradingTerminal::exchange_symbol_display_name(&sym);
    terminal.exchange_symbols = vec![sym];

    let _task = terminal.apply_one_shot_placement_outcome(
        one_shot_outcome_context("#660"),
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::TransportUnknown,
            status: "exchange request failed".to_string(),
            is_error: true,
            refresh_account: true,
        },
    );
    let (message, _) = terminal.order_status.clone().expect("status should be set");
    assert!(message.contains(&format!("placement status unknown for {label}")));
    assert!(!message.contains("#660"));

    let context = one_shot_outcome_context("#660");
    let request_id = begin_one_shot_status_request(&mut terminal, &context);
    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("open")),
    );
    let (message, _) = terminal.order_status.clone().expect("status should be set");
    assert!(message.contains(&format!("confirmed by orderStatus for {label}")));
    assert!(!message.contains("#660"));
}

#[test]
fn one_shot_results_are_ignored_after_account_switch() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());

    let _task = terminal.apply_one_shot_placement_outcome(
        one_shot_context(),
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::AcceptedResting,
            status: "Resting (oid 42)".to_string(),
            is_error: false,
            refresh_account: true,
        },
    );

    assert!(terminal.order_status.is_none());

    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);
    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("open")),
    );

    assert!(terminal.order_status.is_none());
}

#[test]
fn nuke_results_aggregate_until_all_children_settle() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(7, 2, 1));

    let _task = terminal.handle_nuke_result(
        7,
        nuke_context("BTC"),
        Ok(exchange_response(vec![serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100",
                "oid": 42_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE progress: 1/2 confirmed; 1 skipped");
    assert!(terminal.pending_nuke_execution.is_some());

    let _task = terminal.handle_nuke_result(
        7,
        nuke_context("ETH"),
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order rejected"
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert_eq!(
        message,
        "NUKE completed: 1/2 confirmed; 1 failed; 1 skipped"
    );
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn duplicate_nuke_direct_result_does_not_settle_two_children() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(8, 2, 0));
    let first_context = nuke_context("BTC");

    for _ in 0..2 {
        let _task = terminal.handle_nuke_result(
            8,
            first_context.clone(),
            Ok(exchange_response(vec![serde_json::json!({
                "filled": {
                    "totalSz": "1",
                    "avgPx": "100",
                    "oid": 42_u64
                }
            })])),
        );
    }

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE progress: 1/2 confirmed");
    let pending = terminal
        .pending_nuke_execution
        .as_ref()
        .expect("one unique child must remain pending");
    let rendered = format!("{pending:?}");
    assert!(rendered.contains("completed: 1"));
    assert!(!rendered.contains(&first_context.cloid));

    let _task = terminal.handle_nuke_result(
        8,
        nuke_context("ETH"),
        Ok(exchange_response(vec![serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100",
                "oid": 43_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE completed: 2/2 confirmed");
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn duplicate_nuke_status_result_does_not_settle_two_children() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(10, 2, 0));

    for _ in 0..2 {
        let _task = terminal.handle_nuke_placement_status_result(
            10,
            nuke_context("BTC"),
            Ok(order_status("filled")),
        );
    }

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE progress: 1/2 confirmed");
    assert!(terminal.pending_nuke_execution.is_some());

    let _task = terminal.handle_nuke_placement_status_result(
        10,
        nuke_context("ETH"),
        Ok(order_status("filled")),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE completed: 2/2 confirmed");
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn nuke_uncertain_child_waits_for_order_status_before_aggregating() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(9, 1, 0));

    let _task = terminal.handle_nuke_result(
        9,
        nuke_context("BTC"),
        Err("exchange request failed".to_string()),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("NUKE placement status unknown for BTC"));
    assert!(terminal.pending_nuke_execution.is_some());

    let _task = terminal.handle_nuke_placement_status_result(
        9,
        nuke_context("BTC"),
        Ok(order_status("filled")),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE completed: 1/1 confirmed");
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn nuke_direct_resting_market_child_is_uncertain_not_confirmed() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(13, 1, 0));

    let _task = terminal.handle_nuke_result(
        13,
        nuke_context("BTC"),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("NUKE market order unexpectedly rested for BTC"));
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn nuke_status_open_market_child_is_uncertain_not_confirmed() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(14, 1, 0));

    let _task = terminal.handle_nuke_placement_status_result(
        14,
        nuke_context("BTC"),
        Ok(order_status("open")),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("NUKE market order unexpectedly rested for BTC"));
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn nuke_results_after_account_switch_clear_stale_execution_without_status() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(11, 1, 0));

    let _task = terminal.handle_nuke_result(
        11,
        nuke_context("BTC"),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    assert!(terminal.pending_nuke_execution.is_none());
    assert!(terminal.order_status.is_none());

    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(12, 1, 0));
    let _task = terminal.handle_nuke_placement_status_result(
        12,
        nuke_context("BTC"),
        Ok(order_status("open")),
    );

    assert!(terminal.pending_nuke_execution.is_none());
    assert!(terminal.order_status.is_none());
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

#[test]
fn cancel_result_success_clears_indicator_and_removes_order_locally() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(cancel_exchange_response(vec![serde_json::json!("success")])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_cancel_status_request.is_none());
    let data = terminal.account_data.as_ref().expect("account data");
    assert!(data.open_orders.is_empty());
    assert!(
        terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .active_orders
            .is_empty()
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert_eq!(message, "Cancelled");
    assert!(!is_error);
}

#[test]
fn cancel_result_terminal_error_checks_status_and_keeps_local_order_until_refresh() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order was never placed, already canceled, or filled."
        })])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Cancel may have already resolved"));
    assert!(message.contains("checking orderStatus"));
    assert!(message.contains("refreshing account data"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_result_ambiguous_ack_is_uncertain_and_keeps_local_order() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(malformed_ok_response()),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Cancel status unknown"));
    assert!(message.contains("refreshing account data"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_result_ambiguous_ack_uses_pending_request_after_indicator_expires() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();
    terminal.pending_order_indicators.clear();

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(malformed_ok_response()),
    );

    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Cancel status unknown for order 42"));
    assert!(message.contains("checking orderStatus"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_order_status_open_keeps_cancel_uncertain_and_local_order() {
    let (mut terminal, _pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "BTC".to_string(),
        Ok(order_status("open")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("still uncertain"));
    assert!(message.contains("reports open"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_order_status_error_redacts_sensitive_text() {
    let mut terminal = terminal_with_connected_account();
    arm_pending_cancel_status_request(&mut terminal, TEST_ACCOUNT, 42, "BTC");

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "BTC".to_string(),
        Err("orderStatus request failed: api_key=super-secret".to_string()),
    );

    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Cancel status still uncertain for order 42"));
    assert!(message.contains("api_key=<redacted>"));
    assert!(!message.contains("super-secret"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_order_status_without_matching_pending_request_is_ignored() {
    let (mut terminal, _pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "ETH".to_string(),
        Ok(order_status("canceled")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.order_status.is_none());
    assert!(!terminal.account_loading);
}

#[test]
fn cancel_order_status_terminal_removes_local_order() {
    let (mut terminal, _pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "BTC".to_string(),
        Ok(order_status("canceled")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert!(data.open_orders.is_empty());
    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(
        terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .active_orders
            .is_empty()
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert!(message.contains("Cancel resolved"));
}

#[test]
fn cancel_result_after_account_switch_clears_indicator_without_status() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
    terminal.order_status = None;

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(cancel_exchange_response(vec![serde_json::json!("success")])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(terminal.order_status.is_none());
    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
}

#[test]
fn cancel_result_success_removes_only_matching_symbol_for_same_oid() {
    let mut terminal = terminal_with_connected_account();
    let target_order = open_order_for(42, "flx:BTC");
    let other_order = open_order_for(42, "xyz:BTC");
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_open_orders(vec![target_order.clone(), other_order.clone()]),
    );
    let pending_id =
        terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &target_order);
    assert!(pending_id.is_some());

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(cancel_exchange_response(vec![serde_json::json!("success")])),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert_eq!(data.open_orders[0].coin, other_order.coin);
    assert_eq!(data.open_orders[0].oid, 42);
    assert!(terminal.pending_cancel_status_request.is_none());
}

#[test]
fn cancel_result_success_ignores_open_orders_from_stale_account_snapshot() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();
    terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(cancel_exchange_response(vec![serde_json::json!("success")])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert_eq!(data.open_orders[0].oid, 42);
    assert!(terminal.pending_cancel_status_request.is_none());
}

#[test]
fn cancel_status_terminal_removes_only_matching_symbol_for_same_oid() {
    let mut terminal = terminal_with_connected_account();
    let target_order = open_order_for(42, "flx:BTC");
    let other_order = open_order_for(42, "xyz:BTC");
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_open_orders(vec![target_order.clone(), other_order.clone()]),
    );
    arm_pending_cancel_status_request(&mut terminal, TEST_ACCOUNT, 42, &target_order.coin);

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        target_order.coin,
        Ok(order_status("canceled")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert_eq!(data.open_orders[0].coin, other_order.coin);
    assert_eq!(data.open_orders[0].oid, 42);
    assert!(terminal.pending_cancel_status_request.is_none());
}

#[test]
fn cancel_status_terminal_ignores_open_orders_from_stale_account_snapshot() {
    let (mut terminal, _pending_id) = terminal_with_pending_cancel();
    terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "BTC".to_string(),
        Ok(order_status("canceled")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert_eq!(data.open_orders[0].oid, 42);
    assert!(terminal.pending_cancel_status_request.is_none());
}

#[test]
fn order_result_clears_pending_indicator() {
    let mut terminal = terminal_with_connected_account();
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    let pending_id = terminal.add_pending_order_placement_indicator(
        TEST_ACCOUNT.to_string(),
        "BTC".to_string(),
        true,
        "1".to_string(),
        "100".to_string(),
    );
    assert!(pending_id.is_some());

    let _task = terminal.handle_order_result(
        pending_id,
        one_shot_context(),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn close_position_result_clears_pending_indicator() {
    let mut terminal = terminal_with_connected_account();
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    let pending_id = terminal.add_pending_market_order_placement_indicator(
        TEST_ACCOUNT.to_string(),
        "BTC".to_string(),
        false,
        "1".to_string(),
        "100".to_string(),
    );
    assert!(pending_id.is_some());
    assert!(
        terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .hud_order_animation_active()
    );

    let _task = terminal.handle_close_position_result(
        pending_id,
        one_shot_context(),
        Ok(exchange_response(vec![serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100",
                "oid": 43_u64
            }
        })])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(
        !terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .hud_order_animation_active()
    );
}
