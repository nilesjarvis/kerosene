use super::*;
use crate::annotations::DrawingTool;
use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartInstance, ChartSurfaceId, DetachedChartWindowState};
use crate::message::Message;
use crate::order_execution::{
    OneShotPlacementContext, OrderSurface, PendingNukeExecution, QuickOrderForm,
};
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
    OneShotPlacementContext {
        account_address: TEST_ACCOUNT.to_string(),
        cloid: "0x00000000000000000000000000000000".to_string(),
        surface: OrderSurface::Ticket,
        symbol_key: "BTC".to_string(),
    }
}

fn nuke_context(symbol_key: &str) -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: TEST_ACCOUNT.to_string(),
        cloid: format!("0x{symbol_key:0<32}"),
        surface: OrderSurface::Nuke,
        symbol_key: symbol_key.to_string(),
    }
}

fn terminal_with_connected_account() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
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

fn quick_order_form() -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: "1".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
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
fn exchange_error_responses_do_not_require_account_refresh() {
    let exchange_error = exchange_response(vec![serde_json::json!({
        "error": "Order rejected"
    })]);
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

    assert!(!result_requires_account_refresh(&Ok(exchange_error)));
    assert!(!result_requires_account_refresh(&Ok(later_exchange_error)));
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

    let cancelled =
        classify_execution_result(Ok(exchange_response(vec![serde_json::json!("success")])));
    assert_eq!(cancelled.kind, ExecutionOutcomeKind::Cancelled);
    assert_eq!(cancelled.status, "Cancelled");
    assert!(cancelled.refresh_account);
}

#[test]
fn execution_result_classifier_separates_rejected_ambiguous_and_transport_unknown() {
    let rejected = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "error": "Order rejected"
    })])));
    assert_eq!(rejected.kind, ExecutionOutcomeKind::Rejected);
    assert!(rejected.is_error);
    assert!(!rejected.refresh_account);

    let ambiguous = classify_execution_result(Ok(malformed_ok_response()));
    assert_eq!(ambiguous.kind, ExecutionOutcomeKind::Ambiguous);
    assert_eq!(ambiguous.status, "No response body");
    assert!(!ambiguous.is_error);
    assert!(ambiguous.refresh_account);

    let unknown = classify_execution_result(Err(
        "Exchange request failed: connection closed before response".to_string(),
    ));
    assert_eq!(unknown.kind, ExecutionOutcomeKind::TransportUnknown);
    assert!(unknown.is_error);
    assert!(unknown.refresh_account);
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

    let (message, is_error) = terminal.order_status.expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket placement status unknown for BTC"));
    assert!(message.contains("exchange request failed"));
    assert!(message.contains("checking 0x00000000000000000000000000000000"));
}

#[test]
fn one_shot_order_status_result_normalizes_terminal_statuses() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal
        .handle_one_shot_placement_status_result(one_shot_context(), Ok(order_status("open")));
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert!(message.contains("Ticket placement confirmed by orderStatus for BTC"));

    let _task = terminal
        .handle_one_shot_placement_status_result(one_shot_context(), Ok(order_status("rejected")));
    let (message, is_error) = terminal.order_status.expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket placement rejected according to orderStatus for BTC"));
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

    let _task = terminal
        .handle_one_shot_placement_status_result(one_shot_context(), Ok(order_status("open")));

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
            "resting": {
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

    let (message, is_error) = terminal.order_status.expect("status should be set");
    assert!(is_error);
    assert_eq!(
        message,
        "NUKE completed: 1/2 confirmed; 1 failed; 1 skipped"
    );
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
        Ok(order_status("open")),
    );

    let (message, is_error) = terminal.order_status.expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE completed: 1/1 confirmed");
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
