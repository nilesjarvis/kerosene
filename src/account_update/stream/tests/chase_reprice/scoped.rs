use super::super::fixtures::fill_with_oid;
use super::*;
use crate::account::ChaseRepriceSnapshot;
use crate::read_data_provider::AccountDataRequestContext;
use crate::signing::{ChaseStopPhase, MAX_CHASE_DURATION, MAX_CHASE_REPRICES};

use std::time::{Duration, Instant};

fn pending() -> (TradingTerminal, AccountDataRequestContext) {
    let mut terminal = refresh_ready_terminal();
    let mut data = account_data_with_timestamp(TradingTerminal::now_ms());
    let mut order = open_order(42, Some(false));
    order.sz = "1".into();
    data.open_orders.push(order);
    set_account_data_for_connected_account(&mut terminal, data);
    let mut chase = chase_order();
    chase.lifecycle = ChaseLifecycle::Resting;
    terminal.chase_orders.insert(1, chase);
    let _task = terminal.chase_reprice_to_best_price(1, 101.0);
    let context = terminal.current_account_data_request_context();
    assert!(terminal.account_loading && terminal.account_reconciliation_required);
    (terminal, context)
}

fn snapshot(size: &str) -> ChaseRepriceSnapshot {
    let mut order = open_order(42, Some(false));
    order.sz = size.into();
    ChaseRepriceSnapshot {
        open_orders: vec![order],
        fills: vec![],
    }
}

fn complete(
    terminal: &mut TradingTerminal,
    context: AccountDataRequestContext,
    result: Result<ChaseRepriceSnapshot, String>,
) {
    let _task =
        terminal.handle_chase_reprice_snapshot(1, 42, CONNECTED_ADDRESS.into(), context, result);
}

#[test]
fn chase_scoped_reprice_uses_deduplicated_partial_fills_without_refreshing_other_sections() {
    let (mut terminal, context) = pending();
    let account_timestamp = terminal
        .account_data
        .as_ref()
        .expect("account")
        .fetched_at_ms;
    let revision = terminal.account_data_revision;
    let mut result = snapshot("0.8");
    let fill = fill_with_oid(2_000, 42, "100", "0.4");
    result.fills = vec![fill.clone(), fill];
    complete(&mut terminal, context, Ok(result));
    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
    assert!((chase.filled_size - 0.4).abs() < 1e-12);
    assert!((chase.remaining_size - 0.6).abs() < 1e-12);
    assert_eq!(chase.reprice_count, 1);
    assert!(!terminal.account_loading && !terminal.account_reconciliation_required);
    let data = terminal.account_data.as_ref().expect("account");
    assert_eq!(data.fetched_at_ms, account_timestamp);
    assert_eq!(data.open_orders[0].sz, "1");
    assert_eq!(terminal.account_data_revision, revision);
    // A duplicate delivery cannot dispatch another modify.
    complete(&mut terminal, context, Ok(snapshot("1")));
    assert_eq!(chase_order_by_id(&terminal, 1).reprice_count, 1);
}

#[test]
fn chase_scoped_reprice_preserves_newer_live_fill_progress() {
    let (mut terminal, context) = pending();
    terminal
        .chase_orders
        .get_mut(&1)
        .expect("chase")
        .set_filled_size(0.6);
    let mut result = snapshot("0.8");
    result.fills.push(fill_with_oid(2_000, 42, "100", "0.2"));
    complete(&mut terminal, context, Ok(result));
    let chase = chase_order_by_id(&terminal, 1);
    assert!((chase.filled_size - 0.6).abs() < 1e-12);
    assert!((chase.remaining_size - 0.4).abs() < 1e-12);
}

#[test]
fn chase_scoped_reprice_does_not_reconcile_other_chases_from_a_partial_scope() {
    let (mut terminal, context) = pending();
    let mut other = verifying_chase(ChaseVerificationReason::MissingOrder);
    other.id = 2;
    other.coin = "xyz:GOLD".into();
    other.current_oid = Some(55);
    other.known_oids = vec![55];
    terminal.chase_orders.insert(2, other);
    complete(&mut terminal, context, Ok(snapshot("1")));
    let other = chase_order_by_id(&terminal, 2);
    assert_eq!(other.current_oid, Some(55));
    assert_eq!(
        other.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrder
        }
    );
}

#[test]
fn chase_scoped_reprice_missing_order_never_places_a_replacement() {
    let (mut terminal, context) = pending();
    complete(
        &mut terminal,
        context,
        Ok(ChaseRepriceSnapshot {
            open_orders: vec![],
            fills: vec![],
        }),
    );
    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.current_oid, Some(42));
    assert_eq!(chase.reprice_count, 0);
    assert!(chase.needs_account_verification());
    assert!(terminal.account_loading && terminal.account_reconciliation_required);
    assert_ne!(terminal.current_account_data_request_context(), context);
}

#[test]
fn chase_scoped_reprice_cancels_remaining_exposure_after_target_fill() {
    let (mut terminal, context) = pending();
    let mut result = snapshot("0.1");
    result.fills.push(fill_with_oid(2_000, 42, "100", "1"));
    complete(&mut terminal, context, Ok(result));
    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
    assert_eq!(chase.reprice_count, 0);
}

#[test]
fn chase_scoped_reprice_rechecks_limits_after_verification() {
    for case in ["duration", "count", "drift"] {
        let (mut terminal, context) = pending();
        let chase = terminal.chase_orders.get_mut(&1).expect("chase");
        match case {
            "duration" => {
                chase.started_at = Instant::now() - MAX_CHASE_DURATION - Duration::from_secs(1);
            }
            "count" => chase.reprice_count = MAX_CHASE_REPRICES,
            "drift" => chase.desired_price = Some(106.0),
            _ => unreachable!(),
        }
        let count = chase.reprice_count;
        complete(&mut terminal, context, Ok(snapshot("1")));
        let chase = chase_order_by_id(&terminal, 1);
        assert_eq!(
            chase.lifecycle,
            ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::Canceling { oid: 42 }
            },
            "{case}"
        );
        assert_eq!(chase.reprice_count, count, "{case}");
    }
}

#[test]
fn chase_scoped_reprice_falls_back_on_error_followup_provider_change_or_stale_account() {
    for case in ["error", "followup", "provider", "stale", "stopped", "oid"] {
        let (mut terminal, context) = pending();
        let mut result = Ok(snapshot("1"));
        match case {
            "error" => result = Err("read failed".into()),
            "followup" => terminal.account_refresh_followup_pending = true,
            "provider" => terminal.read_data_provider_generation += 1,
            "stale" => {
                terminal
                    .account_data
                    .as_mut()
                    .expect("account")
                    .fetched_at_ms = 1
            }
            "stopped" => {
                let _task = terminal.stop_chase_by_id(1);
            }
            "oid" => {
                terminal
                    .chase_orders
                    .get_mut(&1)
                    .expect("chase")
                    .current_oid = Some(99)
            }
            _ => unreachable!(),
        }
        complete(&mut terminal, context, result);
        assert_eq!(chase_order_by_id(&terminal, 1).reprice_count, 0, "{case}");
        assert!(
            terminal.account_loading && terminal.account_reconciliation_required,
            "{case}"
        );
        assert_ne!(
            terminal.current_account_data_request_context(),
            context,
            "{case}"
        );
    }
}

#[test]
fn chase_scoped_reprice_old_generation_and_other_account_cannot_unlock_loading() {
    for other_account in [false, true] {
        let (mut terminal, context) = pending();
        if other_account {
            terminal.connected_address = Some("other-account".into());
        } else {
            terminal.begin_account_data_request_context();
        }
        let expected_generation = terminal.account_data_request_generation;
        complete(&mut terminal, context, Ok(snapshot("1")));
        assert!(terminal.account_loading && terminal.account_reconciliation_required);
        assert_eq!(
            terminal.account_data_request_generation,
            expected_generation
        );
        assert_eq!(chase_order_by_id(&terminal, 1).reprice_count, 0);
    }
}
