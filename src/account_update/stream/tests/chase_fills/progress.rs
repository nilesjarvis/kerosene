use super::{ChaseOrder, chase_order, chase_order_by_id, fill_with_oid, terminal_with_chase_fills};

#[test]
fn chase_fill_reconciliation_updates_filled_and_remaining_size() {
    let mut terminal =
        terminal_with_chase_fills(chase_order(), vec![fill_with_oid(1_001, 42, "100", "0.1")]);

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert!((chase.filled_size - 0.1).abs() < f64::EPSILON);
    assert!((chase.remaining_size - 0.9).abs() < f64::EPSILON);
}

#[test]
fn chase_fill_reconciliation_sums_known_reprice_oids() {
    let mut chase = chase_order();
    chase.known_oids.push(43);
    let mut terminal = terminal_with_chase_fills(
        chase,
        vec![
            fill_with_oid(1_001, 42, "100", "0.1"),
            fill_with_oid(1_002, 43, "101", "0.2"),
        ],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert!((chase.filled_size - 0.3).abs() < 1e-12);
    assert!((chase.remaining_size - 0.7).abs() < 1e-12);
}

#[test]
fn chase_fill_reconciliation_ignores_matching_oids_before_local_chase_start() {
    let mut chase = chase_order();
    chase.started_at_ms = 120_000;
    chase.fill_cutoff_ms_by_oid =
        vec![(42, ChaseOrder::adopted_fill_cutoff_ms(chase.started_at_ms))];
    let mut terminal = terminal_with_chase_fills(
        chase,
        vec![
            fill_with_oid(10_000, 42, "100", "0.4"),
            fill_with_oid(120_001, 42, "101", "0.1"),
        ],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert!((chase.filled_size - 0.1).abs() < 1e-12);
    assert!((chase.remaining_size - 0.9).abs() < 1e-12);
}

#[test]
fn chase_fill_reconciliation_deduplicates_matching_oid_fills() {
    let mut chase = chase_order();
    chase.started_at_ms = 120_000;
    chase.fill_cutoff_ms_by_oid =
        vec![(42, ChaseOrder::adopted_fill_cutoff_ms(chase.started_at_ms))];
    let duplicate = fill_with_oid(10_000, 42, "100", "0.4");
    let mut terminal = terminal_with_chase_fills(
        chase,
        vec![
            duplicate.clone(),
            duplicate,
            fill_with_oid(120_001, 42, "101", "0.1"),
        ],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert!((chase.filled_size - 0.1).abs() < 1e-12);
    assert!((chase.remaining_size - 0.9).abs() < 1e-12);
}

#[test]
fn chase_fill_reconciliation_credits_locally_placed_oid_despite_clock_skew() {
    let mut chase = chase_order();
    chase.started_at_ms = 120_000;
    chase.fill_cutoff_ms_by_oid.clear();
    let mut terminal =
        terminal_with_chase_fills(chase, vec![fill_with_oid(119_500, 42, "100", "0.4")]);

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert!((chase.filled_size - 0.4).abs() < 1e-12);
    assert!((chase.remaining_size - 0.6).abs() < 1e-12);
}
