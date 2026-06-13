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
fn live_fill_reconciliation_ignores_chase_for_other_connected_account() {
    let mut chase = chase_order();
    chase.account_address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    let mut terminal =
        terminal_with_chase_fills(chase, vec![fill_with_oid(1_001, 42, "100", "0.4")]);
    terminal.connected_address = Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string());

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 0.0);
    assert_eq!(chase.remaining_size, 1.0);
    assert!(terminal.advanced_order_history.is_empty());
}

#[test]
fn chase_fill_reconciliation_ignores_mismatched_account_snapshot_owner() {
    let mut terminal =
        terminal_with_chase_fills(chase_order(), vec![fill_with_oid(1_001, 42, "100", "0.4")]);
    terminal.account_data_address = Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string());

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 0.0);
    assert_eq!(chase.remaining_size, 1.0);
    assert!(terminal.advanced_order_history.is_empty());
}

#[test]
fn chase_fill_reconciliation_requires_matching_coin_and_side() {
    let mut chase = chase_order();
    chase.coin = "flx:BTC".to_string();
    let mut wrong_coin = fill_with_oid(1_001, 42, "100", "0.4");
    wrong_coin.coin = "BTC".to_string();
    let mut wrong_side = fill_with_oid(1_002, 42, "100", "0.3");
    wrong_side.coin = "flx:BTC".to_string();
    wrong_side.side = "A".to_string();
    let mut matching = fill_with_oid(1_003, 42, "100", "0.2");
    matching.coin = "flx:BTC".to_string();
    let mut terminal = terminal_with_chase_fills(chase, vec![wrong_coin, wrong_side, matching]);

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert!((chase.filled_size - 0.2).abs() < 1e-12);
    assert!((chase.remaining_size - 0.8).abs() < 1e-12);
}

#[test]
fn chase_fill_reconciliation_distinguishes_native_and_hip3_same_oid() {
    let mut native_fill = fill_with_oid(1_001, 42, "100", "0.1");
    native_fill.coin = "BTC".to_string();
    let mut hip3_fill = fill_with_oid(1_002, 42, "100", "0.4");
    hip3_fill.coin = "flx:BTC".to_string();

    let mut native_terminal = terminal_with_chase_fills(chase_order(), vec![hip3_fill.clone()]);
    let _task = native_terminal.reconcile_chase_fills_from_account();
    let chase = chase_order_by_id(&native_terminal, 1);
    assert_eq!(chase.filled_size, 0.0);
    assert_eq!(chase.remaining_size, 1.0);

    let mut hip3_chase = chase_order();
    hip3_chase.coin = "flx:BTC".to_string();
    let mut hip3_terminal = terminal_with_chase_fills(hip3_chase, vec![native_fill, hip3_fill]);
    let _task = hip3_terminal.reconcile_chase_fills_from_account();
    let chase = chase_order_by_id(&hip3_terminal, 1);
    assert!((chase.filled_size - 0.4).abs() < 1e-12);
    assert!((chase.remaining_size - 0.6).abs() < 1e-12);
}

#[test]
fn chase_sell_fill_reconciliation_requires_ask_side() {
    let mut chase = chase_order();
    chase.is_buy = false;
    let mut wrong_side = fill_with_oid(1_001, 42, "100", "0.4");
    wrong_side.side = "B".to_string();
    let mut matching = fill_with_oid(1_002, 42, "100", "0.2");
    matching.side = "A".to_string();
    let mut terminal = terminal_with_chase_fills(chase, vec![wrong_side, matching]);

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert!((chase.filled_size - 0.2).abs() < 1e-12);
    assert!((chase.remaining_size - 0.8).abs() < 1e-12);
}

#[test]
fn chase_fill_reconciliation_requires_exact_spot_and_outcome_symbols() {
    for (expected_coin, mismatched_coin) in [("@107", "BTC"), ("#950", "@107")] {
        let mut chase = chase_order();
        chase.coin = expected_coin.to_string();
        let mut mismatched = fill_with_oid(1_001, 42, "100", "1.0");
        mismatched.coin = mismatched_coin.to_string();
        let mut terminal = terminal_with_chase_fills(chase, vec![mismatched]);

        let _task = terminal.reconcile_chase_fills_from_account();

        let chase = chase_order_by_id(&terminal, 1);
        assert_eq!(chase.filled_size, 0.0, "{expected_coin}");
        assert_eq!(chase.remaining_size, 1.0, "{expected_coin}");
    }
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
