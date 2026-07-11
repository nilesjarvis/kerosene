use super::super::twap_cancel_target_matches;
use super::fixtures::{exchange_response_from_value, test_twap, twap_by_id};
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
        Ok(cancel_success_response()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.cancel_retries, 0);
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
    }

    let _task = terminal.handle_twap_unexpected_cancel_result(
        1,
        Some(OID),
        Some(CLOID.to_string()),
        Err("cancel request failed: token=super-secret".to_string()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.status, TwapStatus::Error);
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
    }

    let task =
        terminal.handle_twap_unexpected_cancel_retry_due(1, Some(OID), Some(CLOID.to_string()), 1);

    assert_eq!(task.units(), 1);
}

#[test]
fn unexpected_cancel_retry_due_ignores_stale_attempt() {
    let mut terminal = terminal_with_unexpected_cancel();
    {
        let twap = terminal.twap_orders.get_mut(&1).expect("twap");
        twap.cancel_retries = 2;
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

fn terminal_with_unexpected_cancel() -> TradingTerminal {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut twap = test_twap(1, CLOID, now);
    twap.pending_op = Some(TwapPendingOp::CancelUnexpectedResting {
        oid: Some(OID),
        cloid: Some(CLOID.to_string()),
    });
    twap.cancel_retries = 0;
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
