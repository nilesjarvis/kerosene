use super::{chase, chase_by_id};
use crate::app_state::TradingTerminal;
use crate::order_execution::PendingOrderAction;
use crate::signing::ChaseLifecycle;

fn terminal_ready_for_chase_place() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    terminal
}

#[test]
fn chase_place_uses_unfilled_residual_size() {
    let mut terminal = terminal_ready_for_chase_place();
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids.clear();
    chase.filled_size = 0.1;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Placing);
    assert!((chase.remaining_size - 0.9).abs() < 1e-12);
}

#[test]
fn chase_place_assigns_unique_cloid_per_place_attempt() {
    let mut terminal = terminal_ready_for_chase_place();
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids.clear();
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Placing);
    assert_eq!(chase.place_attempt_count, 1);
    assert!(
        chase
            .current_cloid
            .as_deref()
            .is_some_and(|cloid| { cloid.starts_with("0x") && cloid.len() == 34 })
    );
}

#[test]
fn startup_chase_removal_before_exchange_request_clears_pending_action() {
    let mut terminal = terminal_ready_for_chase_place();
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids.clear();
    terminal.chase_orders.insert(1, chase);
    terminal.selected_chase_id = Some(1);
    terminal.pending_order_action = Some(PendingOrderAction::ChaseBuy);

    let _task = terminal.chase_place_at_best(1, f64::NAN);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
}
