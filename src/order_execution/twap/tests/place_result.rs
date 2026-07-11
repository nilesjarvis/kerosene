use super::super::{
    TwapAccountRefresh, TwapExchangeErrorAction, classify_twap_exchange_error,
    twap_place_result_refresh_policy,
};
use super::fixtures::{exchange_response, exchange_response_from_value, pending_twap, twap_by_id};
use crate::app_state::TradingTerminal;
use crate::signing::ExchangeResponse;
use crate::twap_state::{TwapChildStatus, TwapPauseReason, TwapPendingOp, TwapStatus};
use std::time::Instant;

#[test]
fn twap_place_refresh_policy_reconciles_only_unknown_or_terminal_results() {
    let unknown: Result<ExchangeResponse, String> =
        Err("Exchange request failed after submit".to_string());
    assert_eq!(
        twap_place_result_refresh_policy(&unknown),
        TwapAccountRefresh::Immediate
    );

    let rejected = Ok(exchange_response(serde_json::json!({
        "error": "Order must have minimum value of $10"
    })));
    assert_eq!(
        twap_place_result_refresh_policy(&rejected),
        TwapAccountRefresh::None
    );

    let filled = Ok(exchange_response(serde_json::json!({
        "filled": {
            "totalSz": "1.25",
            "avgPx": "100",
            "oid": 77_u64
        }
    })));
    assert_eq!(
        twap_place_result_refresh_policy(&filled),
        TwapAccountRefresh::OnTerminal
    );

    let ambiguous: Result<ExchangeResponse, String> = Ok(exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": "schema-shifted"
                }
            }
        }),
        "ambiguous exchange response should deserialize",
    ));
    assert_eq!(
        twap_place_result_refresh_policy(&ambiguous),
        TwapAccountRefresh::Immediate
    );

    assert!(!TwapAccountRefresh::OnTerminal.should_refresh(false));
    assert!(TwapAccountRefresh::OnTerminal.should_refresh(true));
    assert!(TwapAccountRefresh::Immediate.should_refresh(false));
}

#[test]
fn twap_exchange_error_classification_separates_retryable_and_terminal_errors() {
    assert_eq!(
        classify_twap_exchange_error("Error: 429 Too Many Requests"),
        TwapExchangeErrorAction::Retry(TwapPauseReason::RateLimited)
    );
    assert_eq!(
        classify_twap_exchange_error("Error: Order must have minimum value of $10"),
        TwapExchangeErrorAction::Terminal
    );
    assert_eq!(
        classify_twap_exchange_error("Error: Order could not immediately match"),
        TwapExchangeErrorAction::ConsumeSlice
    );
}

#[test]
fn conflicting_slice_result_waits_for_cloid_reconciliation() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
    terminal
        .twap_orders
        .insert(1, pending_twap(1, "0xaaa", now));

    let response = exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [
                        {
                            "filled": {
                                "totalSz": "0.5",
                                "avgPx": "100",
                                "oid": 77_u64
                            }
                        },
                        {"error": "conflicting rejection"}
                    ]
                }
            }
        }),
        "conflicting slice response should deserialize",
    );
    let _task = terminal.handle_twap_slice_result(1, 1, 0, Ok(response));
    terminal.reconcile_twap_fills_for_account_after_refresh("0xabc", &[]);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::StatusUnknown));
    assert_eq!(twap.status_check_cloid.as_deref(), Some("0xaaa"));
    assert_eq!(twap.filled_size, 0.0);
    assert_eq!(twap.remaining_size, 1.0);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
    assert_eq!(twap.child_orders[0].oid, None);
    assert_eq!(twap.child_orders[0].filled_size, 0.0);
    assert_eq!(twap.child_orders[0].avg_price, None);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn non_conflicting_fill_without_oid_preserves_existing_fill_accounting() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .twap_orders
        .insert(1, pending_twap(1, "0xaaa", now));

    let response = exchange_response(serde_json::json!({
        "filled": {
            "totalSz": "0.5",
            "avgPx": "100"
        }
    }));
    assert!(response.is_ambiguous_order_result());
    assert!(!response.has_conflicting_order_effect());

    let _task = terminal.handle_twap_slice_result(1, 1, 0, Ok(response));

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.remaining_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
    assert_eq!(twap.status_check_cloid, None);
    assert!(!terminal.account_loading);
}

#[test]
fn retryable_slice_error_pauses_active_twap_for_retry() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .twap_orders
        .insert(1, pending_twap(1, "0xaaa", now));

    let _task = terminal.handle_twap_slice_result(
        1,
        1,
        0,
        Ok(exchange_response(serde_json::json!({
            "error": "429 Too Many Requests"
        }))),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::RateLimited));
    assert_eq!(twap.pending_op, None);
    assert!(twap.retry_slice.is_some());
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Retrying);
}

#[test]
fn duplicate_slice_result_cannot_consume_a_settled_attempt() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .twap_orders
        .insert(1, pending_twap(1, "0xaaa", now));

    let _task = terminal.handle_twap_slice_result(
        1,
        1,
        0,
        Ok(exchange_response(serde_json::json!({
            "error": "429 Too Many Requests"
        }))),
    );
    let settled_status = terminal.order_status.clone();

    let _task = terminal.handle_twap_slice_result(
        1,
        1,
        0,
        Ok(exchange_response(serde_json::json!({
            "filled": {
                "totalSz": "0.5",
                "avgPx": "100",
                "oid": 77_u64
            }
        }))),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::RateLimited));
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.filled_size, 0.0);
    assert_eq!(twap.remaining_size, 1.0);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Retrying);
    assert_eq!(twap.child_orders[0].retry_count, 1);
    assert_eq!(terminal.order_status, settled_status);
}

#[test]
fn stale_slice_result_requires_current_index_and_retry_count() {
    for (stale_index, stale_retry_count) in [(1, 1), (2, 0)] {
        let now = Instant::now();
        let mut terminal = TradingTerminal::boot().0;
        let mut twap = pending_twap(1, "0xbbb", now);
        twap.slices_attempted = 2;
        if let Some(TwapPendingOp::Place(slice)) = twap.pending_op.as_mut() {
            slice.index = 2;
            slice.retry_count = 1;
        }
        twap.child_orders[0].index = 2;
        twap.child_orders[0].retry_count = 1;
        terminal.twap_orders.insert(1, twap);
        terminal.connected_address = Some("0xabc".to_string());
        terminal.account_loading = false;
        terminal.account_reconciliation_required = false;
        terminal.order_status = Some(("Current slice attempt pending".to_string(), false));

        let _task = terminal.handle_twap_slice_result(
            1,
            stale_index,
            stale_retry_count,
            Err("stale transport result".to_string()),
        );

        let twap = twap_by_id(&terminal, 1);
        assert!(matches!(
            twap.pending_op.as_ref(),
            Some(TwapPendingOp::Place(slice))
                if slice.index == 2 && slice.retry_count == 1
        ));
        assert_eq!(twap.filled_size, 0.0);
        assert_eq!(twap.remaining_size, 1.0);
        assert_eq!(twap.child_orders[0].status, TwapChildStatus::Pending);
        assert!(!terminal.account_loading);
        assert!(!terminal.account_reconciliation_required);
        assert_eq!(
            terminal.order_status,
            Some(("Current slice attempt pending".to_string(), false))
        );
    }
}

#[test]
fn stopped_in_flight_twap_does_not_retry_after_retryable_slice_error() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .twap_orders
        .insert(1, pending_twap(1, "0xaaa", now));

    let _task = terminal.stop_twap(1);

    let _task = terminal.handle_twap_slice_result(
        1,
        1,
        0,
        Ok(exchange_response(serde_json::json!({
            "error": "429 Too Many Requests"
        }))),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.retry_slice, None);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::NoFill);
    assert!(
        twap.child_orders[0]
            .exchange_summary
            .contains("429 Too Many Requests")
    );
    assert_eq!(
        terminal
            .order_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("TWAP stopped", false))
    );
    assert!(
        terminal
            .advanced_order_history
            .iter()
            .any(|entry| entry.source_id == 1 && entry.status == "Stopped")
    );
}
