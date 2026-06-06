use super::*;
use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::order_execution::{OneShotPlacementContext, OrderSurface, PendingNukeExecution};

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
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        cloid: "0x00000000000000000000000000000000".to_string(),
        surface: OrderSurface::Ticket,
        symbol_key: "BTC".to_string(),
    }
}

fn nuke_context(symbol_key: &str) -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        cloid: format!("0x{symbol_key:0<32}"),
        surface: OrderSurface::Nuke,
        symbol_key: symbol_key.to_string(),
    }
}

fn order_status(status: &str) -> OrderStatusResult {
    OrderStatusResult {
        status: status.to_string(),
        oid: Some(42),
        cloid: Some("0x00000000000000000000000000000000".to_string()),
        raw_summary: format!("{status} (oid 42)"),
    }
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
    let (mut terminal, _) = TradingTerminal::boot();

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
    let (mut terminal, _) = TradingTerminal::boot();

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
fn nuke_results_aggregate_until_all_children_settle() {
    let (mut terminal, _) = TradingTerminal::boot();
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
    let (mut terminal, _) = TradingTerminal::boot();
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
