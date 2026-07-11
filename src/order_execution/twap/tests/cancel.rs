use super::super::twap_cancel_target_matches;
use super::fixtures::{exchange_response_from_value, test_twap, twap_by_id, user_fill};
use crate::app_state::TradingTerminal;
use crate::twap_state::{
    TWAP_MAX_UNEXPECTED_CANCEL_RETRIES, TwapChildStatus, TwapPauseReason, TwapPendingOp, TwapStatus,
};

use std::time::{Duration, Instant};

const CLOID: &str = "0x1234567890abcdef1234567890abcdef";
const OID: u64 = 42;

#[test]
fn twap_cancel_target_matches_by_oid_or_cloid() {
    assert!(twap_cancel_target_matches(
        Some(42),
        Some("0xabc"),
        Some(42),
        None
    ));
    assert!(twap_cancel_target_matches(
        None,
        Some("0xabc"),
        None,
        Some("0xabc")
    ));
    assert!(!twap_cancel_target_matches(
        Some(42),
        Some("0xabc"),
        Some(43),
        Some("0xdef")
    ));
}

#[test]
fn confirmed_unexpected_child_cancel_clears_pending_cancel() {
    let mut terminal = terminal_with_unexpected_cancel();

    let _task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(cancel_success_response()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.cancel_retries, 0);
    assert_eq!(twap.unexpected_cancel_pending_attempt, None);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::UnexpectedRestingCancelled
    );
}

#[test]
fn ambiguous_ok_unexpected_child_cancel_stays_pending_for_retry() {
    let mut terminal = terminal_with_unexpected_cancel();
    let before = Instant::now();

    let task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(empty_cancel_response()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(
        twap.pending_op,
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: Some(OID),
            cloid: Some(CLOID.to_string()),
        })
    );
    assert_eq!(twap.cancel_retries, 1);
    assert_eq!(twap.unexpected_cancel_pending_attempt, None);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::UnexpectedResting
    );
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::UnexpectedResting));
    let paused_until = twap
        .paused_until
        .expect("retry should record backoff deadline");
    assert!(
        paused_until.saturating_duration_since(before) >= Duration::from_secs(1),
        "retry deadline should not be immediate"
    );
    assert_eq!(task.units(), 2);
    assert!(
        twap.child_orders[0]
            .exchange_summary
            .contains("OK (no statuses)")
    );
}

#[test]
fn conflicting_unexpected_child_cancel_stays_pending_for_retry() {
    let mut terminal = terminal_with_unexpected_cancel();

    let task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(conflicting_cancel_response()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(
        twap.pending_op,
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: Some(OID),
            cloid: Some(CLOID.to_string()),
        })
    );
    assert_eq!(twap.cancel_retries, 1);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::UnexpectedResting
    );
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::UnexpectedResting));
    assert_eq!(task.units(), 2);
}

#[test]
fn transport_unexpected_child_cancel_redacts_child_summary_before_retry() {
    let mut terminal = terminal_with_unexpected_cancel();

    let task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Err("cancel request failed: api_key=super-secret".to_string()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(
        twap.pending_op,
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: Some(OID),
            cloid: Some(CLOID.to_string()),
        })
    );
    assert_eq!(twap.cancel_retries, 1);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::UnexpectedResting
    );
    assert!(
        twap.child_orders[0]
            .exchange_summary
            .contains("api_key=<redacted>")
    );
    assert!(
        !twap.child_orders[0]
            .exchange_summary
            .contains("super-secret")
    );
    assert_eq!(task.units(), 2);
}

#[test]
fn exhausted_transport_unexpected_child_cancel_redacts_error_event() {
    let mut terminal = terminal_with_unexpected_cancel();
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = TWAP_MAX_UNEXPECTED_CANCEL_RETRIES - 1;
        twap.unexpected_cancel_pending_attempt = Some(TWAP_MAX_UNEXPECTED_CANCEL_RETRIES - 1);
    }

    let _task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        TWAP_MAX_UNEXPECTED_CANCEL_RETRIES - 1,
        Err("cancel request failed: token=super-secret".to_string()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.status, TwapStatus::Error);
    assert_eq!(twap.unexpected_cancel_pending_attempt, None);
    let event = twap.events.last().expect("error event");
    assert!(event.is_error);
    assert!(event.message.contains("Cancel status unknown"));
    assert!(event.message.contains("token=<redacted>"));
    assert!(!event.message.contains("super-secret"));
}

#[test]
fn unexpected_cancel_retry_due_revalidates_current_pending_cancel() {
    let mut terminal = terminal_with_unexpected_cancel();
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = 1;
        twap.unexpected_cancel_pending_attempt = None;
    }

    let task =
        terminal.handle_twap_unexpected_cancel_retry_due(1, Some(OID), Some(CLOID.to_string()), 1);

    assert_eq!(task.units(), 1);
    assert_eq!(
        twap_by_id(&terminal, 1).unexpected_cancel_pending_attempt,
        Some(1)
    );
}

#[test]
fn duplicate_unexpected_cancel_retry_due_dispatches_once() {
    let mut terminal = terminal_with_unexpected_cancel();
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = 1;
        twap.unexpected_cancel_pending_attempt = None;
    }

    let first_task =
        terminal.handle_twap_unexpected_cancel_retry_due(1, Some(OID), Some(CLOID.to_string()), 1);
    let duplicate_task =
        terminal.handle_twap_unexpected_cancel_retry_due(1, Some(OID), Some(CLOID.to_string()), 1);

    assert_eq!(first_task.units(), 1);
    assert_eq!(duplicate_task.units(), 0);
    assert_eq!(
        twap_by_id(&terminal, 1).unexpected_cancel_pending_attempt,
        Some(1)
    );
}

#[test]
fn current_unexpected_cancel_retry_result_settles_attempt() {
    let mut terminal = terminal_with_unexpected_cancel();
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = 1;
        twap.unexpected_cancel_pending_attempt = Some(1);
    }

    let _task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        1,
        Ok(cancel_success_response()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.cancel_retries, 0);
    assert_eq!(twap.unexpected_cancel_pending_attempt, None);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::UnexpectedRestingCancelled
    );
}

#[test]
fn unexpected_cancel_retry_due_ignores_stale_attempt() {
    let mut terminal = terminal_with_unexpected_cancel();
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = 2;
        twap.unexpected_cancel_pending_attempt = None;
    }

    let task =
        terminal.handle_twap_unexpected_cancel_retry_due(1, Some(OID), Some(CLOID.to_string()), 1);

    assert_eq!(task.units(), 0);
}

#[test]
fn unexpected_cancel_retry_due_ignores_mismatched_target() {
    let mut terminal = terminal_with_unexpected_cancel();
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = 1;
        twap.unexpected_cancel_pending_attempt = None;
    }

    let task = terminal.handle_twap_unexpected_cancel_retry_due(
        1,
        Some(OID + 1),
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        1,
    );

    assert_eq!(task.units(), 0);
}

#[test]
fn unexpected_cancel_retry_due_ignores_terminal_twap() {
    let mut terminal = terminal_with_unexpected_cancel();
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = 1;
        twap.unexpected_cancel_pending_attempt = None;
        twap.status = TwapStatus::Error;
    }

    let task =
        terminal.handle_twap_unexpected_cancel_retry_due(1, Some(OID), Some(CLOID.to_string()), 1);

    assert_eq!(task.units(), 0);
}

#[test]
fn stale_unexpected_child_cancel_result_is_noop_for_mismatched_target() {
    let mut terminal = terminal_with_unexpected_cancel();
    let original_next_slice_due;
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.status = TwapStatus::Paused;
        twap.pause_reason = Some(TwapPauseReason::UnexpectedResting);
        original_next_slice_due = twap.next_slice_due;
    }

    let _task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID + 1),
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        0,
        Ok(cancel_success_response()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(
        twap.pending_op,
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: Some(OID),
            cloid: Some(CLOID.to_string()),
        })
    );
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::UnexpectedResting));
    assert_eq!(twap.next_slice_due, original_next_slice_due);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::UnexpectedResting
    );
}

#[test]
fn duplicate_unexpected_cancel_result_cannot_consume_retry_budget_twice() {
    let mut terminal = terminal_with_unexpected_cancel();

    let first_task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(empty_cancel_response()),
    );
    let event_count = twap_by_id(&terminal, 1).events.len();
    let paused_until = twap_by_id(&terminal, 1).paused_until;

    let duplicate_task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(empty_cancel_response()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(first_task.units(), 2);
    assert_eq!(duplicate_task.units(), 0);
    assert_eq!(twap.cancel_retries, 1);
    assert_eq!(twap.unexpected_cancel_pending_attempt, None);
    assert_eq!(twap.events.len(), event_count);
    assert_eq!(twap.paused_until, paused_until);
    assert!(matches!(
        twap.pending_op,
        Some(TwapPendingOp::CancelUnexpectedResting { .. })
    ));
}

#[test]
fn duplicate_prior_attempt_error_cannot_falsely_exhaust_cancel_retries() {
    let mut terminal = terminal_with_unexpected_cancel();
    let attempt = TWAP_MAX_UNEXPECTED_CANCEL_RETRIES - 2;
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = attempt;
        twap.unexpected_cancel_pending_attempt = Some(attempt);
    }

    let first_task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        attempt,
        Err("cancel transport outcome unknown".to_string()),
    );
    let event_count = twap_by_id(&terminal, 1).events.len();

    let duplicate_task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        attempt,
        Err("cancel transport outcome unknown".to_string()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(first_task.units(), 2);
    assert_eq!(duplicate_task.units(), 0);
    assert_eq!(twap.cancel_retries, TWAP_MAX_UNEXPECTED_CANCEL_RETRIES - 1);
    assert_ne!(twap.status, TwapStatus::Error);
    assert_eq!(twap.events.len(), event_count);
    assert!(matches!(
        twap.pending_op,
        Some(TwapPendingOp::CancelUnexpectedResting { .. })
    ));
}

#[test]
fn stale_unexpected_cancel_result_cannot_settle_newer_attempt() {
    let mut terminal = terminal_with_unexpected_cancel();

    let _task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(empty_cancel_response()),
    );
    let retry_task =
        terminal.handle_twap_unexpected_cancel_retry_due(1, Some(OID), Some(CLOID.to_string()), 1);
    let event_count = twap_by_id(&terminal, 1).events.len();

    let stale_task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(cancel_success_response()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(retry_task.units(), 1);
    assert_eq!(stale_task.units(), 0);
    assert_eq!(twap.cancel_retries, 1);
    assert_eq!(twap.unexpected_cancel_pending_attempt, Some(1));
    assert_eq!(twap.events.len(), event_count);
    assert!(matches!(
        twap.pending_op,
        Some(TwapPendingOp::CancelUnexpectedResting { .. })
    ));
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::UnexpectedResting
    );
}

#[test]
fn partial_fill_accounting_is_monotonic_across_cancel_result_order() {
    let mut fill_first = terminal_with_attempted_unexpected_cancel();
    fill_first.reconcile_twap_fills_for_account("0xabc", &[user_fill(OID, "0.25", "100")]);
    assert_eq!(
        twap_by_id(&fill_first, 1).child_orders[0].status,
        TwapChildStatus::Filled
    );
    let _task = fill_first.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(cancel_success_response()),
    );

    let mut cancel_first = terminal_with_attempted_unexpected_cancel();
    let _task = cancel_first.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(cancel_success_response()),
    );
    cancel_first.reconcile_twap_fills_for_account("0xabc", &[user_fill(OID, "0.25", "100")]);

    for terminal in [&fill_first, &cancel_first] {
        assert_twap_fill_accounting(terminal, 0.25, 0.75, TwapStatus::WaitingForMarket);
        let twap = twap_by_id(terminal, 1);
        assert_eq!(twap.pending_op, None);
        assert_eq!(twap.unexpected_cancel_pending_attempt, None);
        assert_eq!(twap.cancel_retries, 0);
        assert!(terminal.advanced_order_history.is_empty());
    }

    // Both labels describe a real child effect, but their delivery-order
    // dependence is visible policy and is deliberately only characterized.
    assert_eq!(
        twap_by_id(&fill_first, 1).child_orders[0].status,
        TwapChildStatus::UnexpectedRestingCancelled
    );
    assert_eq!(
        twap_by_id(&cancel_first, 1).child_orders[0].status,
        TwapChildStatus::Filled
    );

    fill_first.reconcile_twap_fills_for_account("0xabc", &[user_fill(OID, "0.25", "100")]);
    cancel_first.reconcile_twap_fills_for_account("0xabc", &[user_fill(OID, "0.25", "100")]);
    assert_twap_fill_accounting(&fill_first, 0.25, 0.75, TwapStatus::WaitingForMarket);
    assert_twap_fill_accounting(&cancel_first, 0.25, 0.75, TwapStatus::WaitingForMarket);
}

#[test]
fn terminal_fill_history_keeps_financial_metrics_across_cancel_result_order() {
    let mut fill_first = terminal_with_attempted_unexpected_cancel();
    prepare_single_child_target(&mut fill_first, 0.5);
    fill_first.reconcile_twap_fills_for_account("0xabc", &[user_fill(OID, "0.5", "100")]);
    let _task = fill_first.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(cancel_success_response()),
    );

    let mut cancel_first = terminal_with_attempted_unexpected_cancel();
    prepare_single_child_target(&mut cancel_first, 0.5);
    let _task = cancel_first.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        0,
        Ok(cancel_success_response()),
    );
    cancel_first.reconcile_twap_fills_for_account("0xabc", &[user_fill(OID, "0.5", "100")]);

    for terminal in [&fill_first, &cancel_first] {
        assert_twap_fill_accounting(terminal, 0.5, 0.0, TwapStatus::Completed);
        let twap = twap_by_id(terminal, 1);
        assert_eq!(twap.pending_op, None);
        assert_eq!(twap.unexpected_cancel_pending_attempt, None);
        assert!(twap.agent_key.as_str().is_empty());

        let entry = terminal
            .advanced_order_history
            .iter()
            .find(|entry| entry.source_id == 1)
            .expect("terminal TWAP should be archived");
        assert_eq!(entry.target_size, 0.5);
        assert_eq!(entry.filled_size, 0.5);
        assert_eq!(entry.remaining_size, 0.0);
        assert_eq!(entry.average_price, Some(100.0));
        assert_eq!(entry.total_fee, 0.01);
        assert_eq!(entry.status, "Completed");
        assert_eq!(entry.children.len(), 1);
        assert_eq!(entry.children[0].filled_size, 0.5);
        assert_eq!(entry.children[0].avg_price, Some(100.0));
        assert_eq!(entry.children[0].fee, 0.01);
    }

    assert_eq!(
        fill_first.advanced_order_history[0].children[0].status,
        "Canceled"
    );
    assert_eq!(
        cancel_first.advanced_order_history[0].children[0].status,
        "Filled"
    );
}

#[test]
fn terminal_fill_cancel_retry_outcomes_refresh_without_scheduling_retry() {
    let retry_outcomes = [
        Ok(empty_cancel_response()),
        Err("cancel transport outcome unknown".to_string()),
    ];

    for result in retry_outcomes {
        let mut terminal = terminal_with_attempted_unexpected_cancel();
        prepare_single_child_target(&mut terminal, 0.5);
        terminal.reconcile_twap_fills_for_account("0xabc", &[user_fill(OID, "0.5", "100")]);
        let history_before = serde_json::to_value(&terminal.advanced_order_history)
            .expect("history should serialize");

        {
            let twap = twap_by_id(&terminal, 1);
            assert_eq!(twap.status, TwapStatus::Completed);
            assert_eq!(twap.unexpected_cancel_pending_attempt, Some(0));
            assert!(twap.agent_key.as_str().is_empty());
        }

        let task = terminal.handle_twap_unexpected_cancel_result(
            1,
            Some(OID),
            Some(CLOID.to_string()),
            0,
            result,
        );

        let twap = twap_by_id(&terminal, 1);
        assert_eq!(task.units(), 1, "only the account refresh should remain");
        assert_eq!(twap.status, TwapStatus::Completed);
        assert_eq!(twap.cancel_retries, 1);
        assert_eq!(twap.unexpected_cancel_pending_attempt, None);
        assert!(twap.agent_key.as_str().is_empty());
        assert_twap_fill_accounting(&terminal, 0.5, 0.0, TwapStatus::Completed);
        assert_eq!(
            serde_json::to_value(&terminal.advanced_order_history)
                .expect("history should serialize"),
            history_before,
            "a delayed retry outcome must not rewrite terminal financial history"
        );

        let retry_task = terminal.handle_twap_unexpected_cancel_retry_due(
            1,
            Some(OID),
            Some(CLOID.to_string()),
            1,
        );
        assert_eq!(retry_task.units(), 0);
        assert_eq!(
            twap_by_id(&terminal, 1).unexpected_cancel_pending_attempt,
            None
        );
    }
}

fn assert_twap_fill_accounting(
    terminal: &TradingTerminal,
    filled_size: f64,
    remaining_size: f64,
    status: TwapStatus,
) {
    let twap = twap_by_id(terminal, 1);
    assert_eq!(twap.filled_size, filled_size);
    assert_eq!(twap.remaining_size, remaining_size);
    assert_eq!(twap.status, status);
    assert_eq!(twap.child_orders[0].filled_size, filled_size);
    assert_eq!(twap.child_orders[0].avg_price, Some(100.0));
    assert_eq!(twap.child_orders[0].fee, 0.01);
    assert_eq!(twap.slices_attempted, 1);
    assert_eq!(twap.slices_sent, 1);
    if status == TwapStatus::WaitingForMarket {
        assert!(twap.next_slice_due > twap.started_at);
    }
}

fn prepare_single_child_target(terminal: &mut TradingTerminal, target_size: f64) {
    let twap = terminal.twap_orders.get_mut(&1).expect("twap");
    twap.target_size = target_size;
    twap.remaining_size = target_size;
    twap.child_orders[0].planned_size = target_size;
}

fn terminal_with_attempted_unexpected_cancel() -> TradingTerminal {
    let mut terminal = terminal_with_unexpected_cancel();
    let twap = terminal.twap_orders.get_mut(&1).expect("twap");
    twap.slices_attempted = 1;
    twap.slices_sent = 1;
    twap.pause_reason = Some(TwapPauseReason::UnexpectedResting);
    terminal
}

fn terminal_with_unexpected_cancel() -> TradingTerminal {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut twap = test_twap(1, CLOID, now);
    twap.pending_op = Some(TwapPendingOp::CancelUnexpectedResting {
        oid: Some(OID),
        cloid: Some(CLOID.to_string()),
    });
    twap.status_check_cloid = None;
    twap.status_check_pending_attempt = None;
    twap.cancel_retries = 0;
    twap.unexpected_cancel_pending_attempt = Some(0);
    twap.child_orders[0].oid = Some(OID);
    twap.child_orders[0].status = TwapChildStatus::UnexpectedResting;
    terminal.twap_orders.insert(1, twap);
    terminal
}

fn cancel_success_response() -> crate::signing::ExchangeResponse {
    exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "cancel",
                "data": {
                    "statuses": ["success"]
                }
            }
        }),
        "cancel success response should deserialize",
    )
}

fn empty_cancel_response() -> crate::signing::ExchangeResponse {
    exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "cancel",
                "data": {
                    "statuses": []
                }
            }
        }),
        "empty cancel response should deserialize",
    )
}

fn conflicting_cancel_response() -> crate::signing::ExchangeResponse {
    exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "cancel",
                "data": {
                    "statuses": [
                        "success",
                        {"error": "Order was never placed, already canceled, or filled."}
                    ]
                }
            }
        }),
        "conflicting cancel response should deserialize",
    )
}
