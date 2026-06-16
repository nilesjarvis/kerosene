use super::{
    CHILD_OID, CLOID, ORIGIN_ADDRESS, SWITCHED_ADDRESS, TwapChildStatus, TwapStatus,
    disable_current_account_refresh, empty_account_data, filled_status, origin_account_terminal,
    pending_twap, reconciliation_twap, switched_account_terminal, test_twap, twap_by_id, user_fill,
};
use crate::account::{AccountData, AccountDataSection};
use crate::config::ReadDataProvider;
use crate::read_data_provider::{AccountDataRequestContext, ReadDataRequestContext};
use crate::twap_state::{TWAP_MAX_RETRY_ATTEMPTS, TwapEventKind};

use std::time::Instant;

fn mark_fills_incomplete(data: &mut AccountData) {
    data.completeness
        .mark_incomplete(AccountDataSection::Fills, "userFills request failed");
}

#[test]
fn twap_status_checks_resolve_origin_account_after_account_switch() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    assert_eq!(
        terminal.twap_origin_address(1).as_deref(),
        Some(ORIGIN_ADDRESS)
    );
}

#[test]
fn stop_twap_does_not_rewrite_terminal_twap() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = test_twap(1, CLOID, now);
    twap.status = TwapStatus::Completed;
    terminal.twap_orders.insert(1, twap);

    let _task = terminal.stop_twap(1);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Completed);
    assert!(!twap.stop_requested);
    assert_eq!(twap.stop_reason, None);
}

#[test]
fn finish_twap_attempt_is_idempotent_for_terminal_stop() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = test_twap(1, CLOID, now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    twap.child_orders[0].status = TwapChildStatus::NoFill;
    terminal.twap_orders.insert(1, twap);

    let _task = terminal.stop_twap(1);
    let event_count = twap_by_id(&terminal, 1).events.len();

    terminal.finish_twap_attempt(1, now);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert_eq!(twap.events.len(), event_count);
}

#[test]
fn twap_reconciliation_uses_fetched_account_scope_after_account_switch() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let fills = vec![user_fill(CHILD_OID, "0.5", "100")];

    terminal.reconcile_twap_fills_for_account(SWITCHED_ADDRESS, &fills);
    assert_eq!(
        terminal.twap_orders.get(&1).map(|twap| twap.filled_size),
        Some(0.0)
    );

    terminal.reconcile_twap_fills_for_account(ORIGIN_ADDRESS, &fills);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.reconciliation_deadline, None);
}

#[test]
fn stale_account_data_loaded_reconciles_twap_without_replacing_current_account() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.account_data = Some(empty_account_data());
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let mut stale_data = empty_account_data();
    stale_data.fills.push(user_fill(CHILD_OID, "0.5", "100"));

    let context = terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);
    let _task =
        terminal.apply_account_data_loaded(ORIGIN_ADDRESS.to_string(), context, Ok(stale_data));

    assert_eq!(
        terminal.connected_address.as_deref(),
        Some(SWITCHED_ADDRESS)
    );
    assert_eq!(
        terminal.account_data.as_ref().map(|data| data.fills.len()),
        Some(0)
    );
    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
}

#[test]
fn twap_reconciliation_result_after_switching_back_does_not_replace_connected_snapshot() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let context = terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);
    let mut reconciliation_data = empty_account_data();
    reconciliation_data
        .fills
        .push(user_fill(CHILD_OID, "0.5", "100"));

    terminal.connected_address = Some(ORIGIN_ADDRESS.to_string());
    terminal.account_data = Some(empty_account_data());
    terminal.account_data_address = Some(ORIGIN_ADDRESS.to_string());
    terminal.account_loading = true;
    terminal.account_refresh_followup_pending = true;
    terminal.account_reconciliation_required = true;

    let _task = terminal.apply_account_data_loaded(
        ORIGIN_ADDRESS.to_string(),
        context,
        Ok(reconciliation_data),
    );

    assert!(terminal.account_loading);
    assert!(terminal.account_refresh_followup_pending);
    assert!(terminal.account_reconciliation_required);
    assert_eq!(
        terminal.account_data.as_ref().map(|data| data.fills.len()),
        Some(0)
    );
    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
}

#[test]
fn stale_off_account_twap_reconciliation_result_is_ignored_when_newer_fetch_exists() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.account_data = Some(empty_account_data());
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let stale_context =
        terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);
    let current_context =
        terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);
    let mut stale_data = empty_account_data();
    stale_data.fills.push(user_fill(CHILD_OID, "0.25", "100"));
    let mut current_data = empty_account_data();
    current_data.fills.push(user_fill(CHILD_OID, "0.5", "100"));

    let _task = terminal.apply_account_data_loaded(
        ORIGIN_ADDRESS.to_string(),
        stale_context,
        Ok(stale_data),
    );
    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.0);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );

    let _task = terminal.apply_account_data_loaded(
        ORIGIN_ADDRESS.to_string(),
        current_context,
        Ok(current_data),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
}

#[test]
fn off_account_twap_reconciliation_error_records_retry_and_can_retry() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let context = terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);

    let _task = terminal.apply_account_data_loaded(
        ORIGIN_ADDRESS.to_string(),
        context,
        Err("429 too many requests".to_string()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.account_reconciliation_retries, 1);
    assert_eq!(twap.status_check_retries, 0);
    assert!(
        twap.events.iter().any(|event| {
            event.kind == TwapEventKind::Retrying
                && event.is_error
                && event.message.contains("retry 1/")
                && event.message.contains("429 too many requests")
        }),
        "reconciliation refresh failure should be visible on the TWAP"
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("TWAP account-fill reconciliation refresh failed")
            })
    );

    let generation_before_retry = terminal
        .account_twap_reconciliation_generations
        .get(ORIGIN_ADDRESS)
        .copied();
    let _task = terminal.retry_twap_reconciliation_account_data(ORIGIN_ADDRESS.to_string());

    assert_eq!(
        terminal
            .account_twap_reconciliation_generations
            .get(ORIGIN_ADDRESS)
            .copied(),
        generation_before_retry.map(|generation| generation.wrapping_add(1))
    );
}

#[test]
fn stale_provider_twap_reconciliation_result_records_retry_with_current_generation() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_key_generation = 2;
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let current_context =
        terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);
    let stale_read_data = ReadDataRequestContext {
        provider: ReadDataProvider::Hydromancer,
        read_data_provider_generation: terminal.read_data_provider_generation,
        hydromancer_key_generation: 1,
    };
    let stale_context = AccountDataRequestContext {
        read_data: stale_read_data,
        scope: current_context.scope,
    };

    let _task = terminal.apply_account_data_loaded(
        ORIGIN_ADDRESS.to_string(),
        stale_context,
        Ok(empty_account_data()),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.account_reconciliation_retries, 1);
    assert_eq!(twap.status_check_retries, 0);
    assert!(
        twap.events.iter().any(|event| {
            event.kind == TwapEventKind::Retrying
                && event
                    .message
                    .contains("read data provider changed before TWAP reconciliation completed")
        }),
        "stale provider/key context should be visible and retryable"
    );
}

#[test]
fn off_account_twap_reconciliation_retries_when_fills_are_incomplete() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let context = terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);
    let mut data = empty_account_data();
    data.fills.push(user_fill(CHILD_OID, "0.5", "100"));
    mark_fills_incomplete(&mut data);

    let _task = terminal.apply_account_data_loaded(ORIGIN_ADDRESS.to_string(), context, Ok(data));

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.0);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );
    assert_eq!(twap.account_reconciliation_retries, 1);
    assert!(
        twap.events.iter().any(|event| {
            event.kind == TwapEventKind::Retrying
                && event.message.contains("Trade history may be incomplete")
        }),
        "incomplete fills should be visible and retried instead of treated as empty fills"
    );
}

#[test]
fn connected_account_refresh_does_not_reconcile_twap_when_fills_are_incomplete() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let context = terminal.current_account_data_request_context();
    let mut data = empty_account_data();
    data.fills.push(user_fill(CHILD_OID, "0.5", "100"));
    mark_fills_incomplete(&mut data);

    let _task = terminal.apply_account_data_loaded(ORIGIN_ADDRESS.to_string(), context, Ok(data));

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.0);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );
    assert_eq!(twap.account_reconciliation_retries, 1);
    assert_eq!(
        terminal.account_data.as_ref().map(|data| data.fills.len()),
        Some(1),
        "the account snapshot can be stored while TWAP fill reconciliation is deferred"
    );
}

#[test]
fn exhausted_twap_reconciliation_error_does_not_start_retry() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    let mut twap = reconciliation_twap(now);
    twap.account_reconciliation_retries = TWAP_MAX_RETRY_ATTEMPTS - 1;
    terminal.twap_orders.insert(1, twap);
    let context = terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);

    let _task = terminal.apply_account_data_loaded(
        ORIGIN_ADDRESS.to_string(),
        context,
        Err("read provider unavailable".to_string()),
    );

    let generation_after_failure = terminal
        .account_twap_reconciliation_generations
        .get(ORIGIN_ADDRESS)
        .copied();
    let _task = terminal.retry_twap_reconciliation_account_data(ORIGIN_ADDRESS.to_string());

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.account_reconciliation_retries, TWAP_MAX_RETRY_ATTEMPTS);
    assert_eq!(twap.status_check_retries, 0);
    assert!(
        twap.events.iter().any(|event| {
            event.kind == TwapEventKind::Error
                && event
                    .message
                    .contains("reconciliation refresh failed after")
        }),
        "exhausted reconciliation refresh failures should be visible"
    );
    assert_eq!(
        terminal
            .account_twap_reconciliation_generations
            .get(ORIGIN_ADDRESS)
            .copied(),
        generation_after_failure
    );
}

#[test]
fn retry_twap_reconciliation_account_data_noops_after_reconciliation_resolves() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let context = terminal.begin_twap_reconciliation_account_data_request_context(ORIGIN_ADDRESS);
    let mut data = empty_account_data();
    data.fills.push(user_fill(CHILD_OID, "0.5", "100"));

    let _task = terminal.apply_account_data_loaded(ORIGIN_ADDRESS.to_string(), context, Ok(data));
    let generation_after_success = terminal
        .account_twap_reconciliation_generations
        .get(ORIGIN_ADDRESS)
        .copied();
    let _task = terminal.retry_twap_reconciliation_account_data(ORIGIN_ADDRESS.to_string());

    assert_eq!(
        terminal
            .account_twap_reconciliation_generations
            .get(ORIGIN_ADDRESS)
            .copied(),
        generation_after_success
    );
}

#[test]
fn running_twap_reconciliation_clears_status_metadata_after_fill() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = reconciliation_twap(now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.paused_until = None;
    twap.next_slice_due = now;
    terminal.twap_orders.insert(1, twap);

    terminal
        .reconcile_twap_fills_for_account(ORIGIN_ADDRESS, &[user_fill(CHILD_OID, "0.5", "100")]);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Running);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.reconciliation_deadline, None);
    assert!(twap.can_schedule_at(now));
}

#[test]
fn running_twap_reconciliation_keeps_status_metadata_for_symbol_mismatch() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = reconciliation_twap(now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.paused_until = None;
    twap.next_slice_due = now;
    terminal.twap_orders.insert(1, twap);

    let mut fill = user_fill(CHILD_OID, "0.5", "100");
    fill.coin = "flx:BTC".to_string();
    terminal.reconcile_twap_fills_for_account(ORIGIN_ADDRESS, &[fill]);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Running);
    assert_eq!(twap.filled_size, 0.0);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert!(twap.reconciliation_deadline.is_some());
}

#[test]
fn running_twap_reconciliation_clears_status_metadata_when_fill_was_already_counted() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = reconciliation_twap(now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.paused_until = None;
    twap.next_slice_due = now;
    twap.filled_size = 0.5;
    twap.remaining_size = 0.5;
    terminal.twap_orders.insert(1, twap);

    terminal
        .reconcile_twap_fills_for_account(ORIGIN_ADDRESS, &[user_fill(CHILD_OID, "0.5", "100")]);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Running);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.remaining_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.reconciliation_deadline, None);
    assert!(twap.can_schedule_at(now));
}

#[test]
fn ambiguous_slice_result_after_account_switch_does_not_refresh_current_account() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    disable_current_account_refresh(&mut terminal);
    terminal.twap_orders.insert(1, pending_twap(1, CLOID, now));

    let _task =
        terminal.handle_twap_slice_result(1, Err("Exchange request failed after submit".into()));

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.account_address, ORIGIN_ADDRESS);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
    assert!(!terminal.account_loading);
    assert!(!terminal.account_reconciliation_required);
}

#[test]
fn ambiguous_slice_result_queues_followup_when_current_account_refresh_is_loading() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.account_loading = true;
    terminal.account_reconciliation_required = false;
    terminal.account_refresh_followup_pending = false;
    terminal.twap_orders.insert(1, pending_twap(1, CLOID, now));

    let _task =
        terminal.handle_twap_slice_result(1, Err("Exchange request failed after submit".into()));

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.account_address, ORIGIN_ADDRESS);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
    assert!(terminal.account_loading);
    assert!(terminal.account_refresh_followup_pending);
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn stopped_transport_unknown_twap_finishes_after_partial_fill_reconciliation() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, pending_twap(1, CLOID, now));

    let _task = terminal.stop_twap(1);
    assert_eq!(twap_by_id(&terminal, 1).status, TwapStatus::Stopping);

    let _task =
        terminal.handle_twap_slice_result(1, Err("Exchange request failed after submit".into()));
    assert_eq!(twap_by_id(&terminal, 1).status, TwapStatus::Stopping);

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        Ok(filled_status(CLOID, CHILD_OID)),
    );
    assert_eq!(twap_by_id(&terminal, 1).status, TwapStatus::Stopping);
    assert_eq!(
        twap_by_id(&terminal, 1).child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );

    terminal
        .reconcile_twap_fills_for_account(ORIGIN_ADDRESS, &[user_fill(CHILD_OID, "0.25", "100")]);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.reconciliation_deadline, None);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
    assert!(
        terminal
            .advanced_order_history
            .iter()
            .any(|entry| entry.source_id == 1 && entry.status == "Stopped")
    );
}
