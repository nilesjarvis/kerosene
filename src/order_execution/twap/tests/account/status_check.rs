use super::{
    CHILD_OID, CLOID, TWAP_MAX_RETRY_ATTEMPTS, TWAP_RECONCILIATION_TIMEOUT, TwapChildStatus,
    TwapStatus, canceled_status, disable_current_account_refresh, filled_status, missing_status,
    open_status, origin_account_terminal, pending_twap, reconciliation_deadline, rejected_status,
    switched_account_terminal, test_twap, twap_by_id,
};
use crate::twap_state::{TwapPauseReason, TwapPendingOp};

use std::time::Instant;

#[test]
fn filled_status_check_arms_reconciliation_deadline() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(filled_status(CLOID, CHILD_OID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.status_check_pending_attempt, None);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );
    let deadline = reconciliation_deadline(twap);
    assert!(deadline > now);
    assert!(deadline <= Instant::now() + TWAP_RECONCILIATION_TIMEOUT);
}

#[test]
fn filled_status_check_after_account_switch_does_not_refresh_current_account() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    disable_current_account_refresh(&mut terminal);
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(filled_status(CLOID, CHILD_OID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.account_address, super::ORIGIN_ADDRESS);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert!(
        twap.reconciliation_deadline.is_some(),
        "filled status must wait for origin-account fills"
    );
    assert!(!terminal.account_loading);
    assert!(!terminal.account_reconciliation_required);
}

#[test]
fn canceled_status_check_waits_for_account_fill_confirmation() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(canceled_status(CLOID, CHILD_OID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.status_check_pending_attempt, None);
    assert_eq!(twap.status_check_retries, 0);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingNoFillConfirmation
    );
    let deadline = reconciliation_deadline(twap);
    assert!(deadline > now);
    assert!(deadline <= Instant::now() + TWAP_RECONCILIATION_TIMEOUT);
    assert_eq!(twap.slices_attempted, 0);
}

#[test]
fn rejected_status_check_still_finishes_without_account_reconciliation() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(rejected_status(CLOID, CHILD_OID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.status_check_pending_attempt, None);
    assert_eq!(twap.reconciliation_deadline, None);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Rejected);
    assert_eq!(twap.status, TwapStatus::WaitingForMarket);
}

#[test]
fn missing_status_check_after_transport_error_retries_before_no_fill() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = test_twap(1, CLOID, now);
    twap.status_check_retries = 1;
    twap.status_check_pending_attempt = Some(1);
    terminal.twap_orders.insert(1, twap);

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        1,
        Ok(missing_status(CLOID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.status_check_retries, 2);
    assert!(twap.paused_until.is_some());
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
    assert_eq!(twap.slices_attempted, 0);
}

#[test]
fn duplicate_status_dispatch_keeps_one_in_flight_owner() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = test_twap(1, CLOID, now);
    twap.status_check_pending_attempt = None;
    terminal.twap_orders.insert(1, twap);

    let first_task = terminal.check_twap_child_status(1, CLOID.to_string());
    let duplicate_task = terminal.check_twap_child_status(1, CLOID.to_string());

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(first_task.units(), 1);
    assert_eq!(duplicate_task.units(), 0);
    assert_eq!(twap.status_check_retries, 0);
    assert_eq!(twap.status_check_pending_attempt, Some(0));
}

#[test]
fn duplicate_status_result_cannot_consume_retry_budget_twice() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let first_task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(missing_status(CLOID)),
    );
    let event_count = twap_by_id(&terminal, 1).events.len();

    let duplicate_task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(missing_status(CLOID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(first_task.units(), 1);
    assert_eq!(duplicate_task.units(), 0);
    assert_eq!(twap.status_check_retries, 1);
    assert_eq!(twap.status_check_pending_attempt, Some(1));
    assert_eq!(twap.events.len(), event_count);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
}

#[test]
fn stale_missing_status_cannot_override_no_fill_confirmation_phase() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(canceled_status(CLOID, CHILD_OID)),
    );
    let reconciliation_deadline = twap_by_id(&terminal, 1).reconciliation_deadline;

    let stale_task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(missing_status(CLOID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(stale_task.units(), 0);
    assert_eq!(twap.status_check_retries, 0);
    assert_eq!(twap.status_check_pending_attempt, None);
    assert_eq!(twap.reconciliation_deadline, reconciliation_deadline);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingNoFillConfirmation
    );
}

#[test]
fn missing_status_exhaustion_after_unknown_slice_fails_closed() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, pending_twap(1, CLOID, now));

    let _task = terminal.handle_twap_slice_result(
        1,
        1,
        0,
        Err("Exchange request failed after submit".to_string()),
    );
    {
        let twap = terminal
            .twap_orders
            .get_mut(&1)
            .expect("twap remains active");
        twap.status_check_retries = TWAP_MAX_RETRY_ATTEMPTS - 1;
        twap.status_check_pending_attempt = Some(TWAP_MAX_RETRY_ATTEMPTS - 1);
    }

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        TWAP_MAX_RETRY_ATTEMPTS - 1,
        Ok(missing_status(CLOID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Error);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.status_check_retries, 0);
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.retry_slice, None);
    assert_eq!(twap.slices_attempted, 0);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
    assert_eq!(
        twap.child_orders[0].exchange_summary,
        missing_status(CLOID).raw_summary
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error
                    && message.contains("status remained missing")
                    && message.contains("check the exchange")
            })
    );
}

#[test]
fn missing_status_check_after_stop_keeps_twap_stopping() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = test_twap(1, CLOID, now);
    twap.status_check_retries = 1;
    twap.status_check_pending_attempt = Some(1);
    terminal.twap_orders.insert(1, twap);

    let _task = terminal.stop_twap(1);
    assert_eq!(twap_by_id(&terminal, 1).status, TwapStatus::Stopping);

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        1,
        Ok(missing_status(CLOID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Stopping);
    assert!(twap.stop_requested);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.status_check_retries, 2);
    assert!(twap.paused_until.is_some());
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
}

#[test]
fn open_status_check_after_stop_keeps_twap_stopping_and_requests_cancel() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let _task = terminal.stop_twap(1);
    assert_eq!(twap_by_id(&terminal, 1).status, TwapStatus::Stopping);

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        0,
        Ok(open_status(CLOID, CHILD_OID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Stopping);
    assert!(twap.stop_requested);
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::UnexpectedResting));
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::UnexpectedResting
    );
    assert!(matches!(
        &twap.pending_op,
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: Some(CHILD_OID),
            cloid: Some(cloid),
        }) if cloid == CLOID
    ));
    assert_eq!(twap.unexpected_cancel_pending_attempt, Some(0));
}

#[test]
fn terminal_status_check_result_is_ignored() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = test_twap(1, CLOID, now);
    twap.status = TwapStatus::Stopped;
    twap.stop_requested = true;
    twap.stop_reason = Some(("TWAP stopped".to_string(), false));
    twap.status_check_retries = 1;
    twap.status_check_pending_attempt = Some(1);
    terminal.twap_orders.insert(1, twap);
    terminal.archive_twap_if_terminal(1);
    let history_before =
        serde_json::to_value(&terminal.advanced_order_history).expect("history should serialize");

    let task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        1,
        Ok(missing_status(CLOID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert!(twap.stop_requested);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.status_check_pending_attempt, Some(1));
    assert_eq!(twap.status_check_retries, 1);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
    assert!(twap.agent_key.as_str().is_empty());
    assert_eq!(task.units(), 0);
    assert_eq!(
        serde_json::to_value(&terminal.advanced_order_history).expect("history should serialize"),
        history_before
    );
}
