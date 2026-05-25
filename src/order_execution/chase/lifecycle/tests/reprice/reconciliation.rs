use super::{
    ChaseLifecycle, ChaseQueuedAction, chase, chase_by_id, exchange_busy_terminal,
    exchange_ready_terminal,
};

#[test]
fn chase_reconciliation_uses_pending_target_price() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.current_price = f64::NAN;
    chase.current_price_wire.clear();
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
    assert_eq!(chase.desired_price, Some(101.0));
}

#[test]
fn chase_reconciliation_queues_size_correction_when_exchange_gate_is_busy() {
    let mut terminal = exchange_busy_terminal();
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::SizeCorrection
        }
    );
}
