use super::{
    ChaseLifecycle, ChaseOrder, ChaseStopPhase, ChaseVerificationReason, chase_order,
    chase_order_by_id, connected_terminal_with_chase_account, fill_with_oid, open_order,
    terminal_with_chase_fills,
};

#[test]
fn chase_fill_reconciliation_removes_fully_filled_chase() {
    let mut terminal =
        terminal_with_chase_fills(chase_order(), vec![fill_with_oid(1_001, 42, "100", "1.0")]);

    let _task = terminal.reconcile_chase_after_account_refresh();

    assert!(terminal.chase_orders.is_empty());
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| !*is_error && message.contains("Chase filled"))
    );
}

#[test]
fn unrelated_hip3_refresh_does_not_archive_filled_chase_with_unknown_open_orders() {
    let mut chase = chase_order();
    chase.coin = "flx:BTC".to_string();
    let mut fill = fill_with_oid(1_001, 42, "100", "1.0");
    fill.coin = "flx:BTC".to_string();
    let mut terminal = terminal_with_chase_fills(chase, vec![fill]);
    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .fetch_scope = crate::account::AccountDataFetchScope::hip3_dex("xyz");

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 1.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrder
        }
    );
    assert!(terminal.advanced_order_history.is_empty());
}

#[test]
fn historical_resting_oid_fills_do_not_complete_adopted_chase() {
    let mut chase = chase_order();
    chase.started_at_ms = 120_000;
    chase.fill_cutoff_ms_by_oid =
        vec![(42, ChaseOrder::adopted_fill_cutoff_ms(chase.started_at_ms))];
    let mut terminal =
        terminal_with_chase_fills(chase, vec![fill_with_oid(10_000, 42, "100", "1.0")]);

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 0.0);
    assert_eq!(chase.remaining_size, 1.0);
    assert!(terminal.chase_orders.contains_key(&1));
}

#[test]
fn live_fill_reconciliation_waits_for_fresh_open_orders_before_removal() {
    let mut terminal =
        terminal_with_chase_fills(chase_order(), vec![fill_with_oid(1_001, 42, "100", "1.0")]);

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 1.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrder
        }
    );
}

#[test]
fn completed_chase_cancels_live_known_order_before_removal() {
    let mut terminal = connected_terminal_with_chase_account(
        chase_order(),
        vec![fill_with_oid(1_001, 42, "100", "1.0")],
        vec![open_order(42, Some(false))],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 1.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
}

#[test]
fn overfilled_chase_preserves_raw_total_and_cancels_live_known_order() {
    let mut chase = chase_order();
    chase.known_oids.push(43);
    let mut terminal = connected_terminal_with_chase_account(
        chase,
        vec![fill_with_oid(1_001, 42, "100", "1.2")],
        vec![open_order(43, Some(false))],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 1.2);
    assert_eq!(chase.remaining_size, 0.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 43 }
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| { *is_error && message.contains("over target") })
    );
}

#[test]
fn completed_chase_with_in_flight_modify_defers_safety_cancel() {
    let mut chase = chase_order();
    chase.lifecycle = ChaseLifecycle::Modifying { oid: 42 };
    let mut terminal = connected_terminal_with_chase_account(
        chase,
        vec![fill_with_oid(1_001, 42, "100", "1.0")],
        vec![open_order(42, Some(false))],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    // Fills are credited, but the safety cancel must wait for the in-flight
    // modify result; forcing it now would put two exchange mutations in
    // flight for the same order.
    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 1.0);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
}

#[test]
fn account_refresh_fill_reconciliation_ignores_chase_for_other_connected_account() {
    let mut chase = chase_order();
    chase.account_address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
    };
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    let mut terminal =
        terminal_with_chase_fills(chase, vec![fill_with_oid(1_001, 42, "100", "1.0")]);
    terminal.connected_address = Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string());

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 0.0);
    assert_eq!(chase.remaining_size, 1.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::VerifyingCancel { oid: 42 }
        }
    );
    assert!(terminal.advanced_order_history.is_empty());
}

#[test]
fn spot_chase_completion_summary_and_archive_use_pair_name() {
    // Spot chase coins are raw "@{index}" pair keys (HYPE/USDC is "@107");
    // the completion status and the archived history summary must show the
    // pair name instead.
    let mut chase = chase_order();
    chase.coin = "@107".to_string();
    chase.is_spot = true;
    let mut fill = fill_with_oid(1_001, 42, "100", "1.0");
    fill.coin = "@107".to_string();
    let mut terminal = terminal_with_chase_fills(chase, vec![fill]);
    terminal.exchange_symbols = vec![crate::api::ExchangeSymbol {
        key: "@107".to_string(),
        ticker: "HYPE".to_string(),
        category: "spot".to_string(),
        display_name: Some("HYPE/USDC".to_string()),
        keywords: Vec::new(),
        asset_index: 10_107,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: crate::api::MarketType::Spot,
        outcome: None,
    }];

    let _task = terminal.reconcile_chase_after_account_refresh();

    assert!(terminal.chase_orders.is_empty());
    let (message, is_error) = terminal
        .order_status
        .clone()
        .expect("completion status should be set");
    assert!(!is_error, "unexpected error status: {message}");
    assert_eq!(message, "Chase filled: BUY 1 HYPE/USDC @ $100");
    let entry = terminal
        .advanced_order_history
        .front()
        .expect("completed chase should be archived");
    assert!(entry.summary.contains("HYPE/USDC"), "{}", entry.summary);
    assert!(!entry.summary.contains("@107"), "{}", entry.summary);
}

#[test]
fn overfilled_chase_completion_error_pushes_toast() {
    // Overfill summaries are errors; they must surface as a toast when the
    // order ticket pane is closed.
    let mut chase = chase_order();
    chase.known_oids.push(43);
    let mut terminal = connected_terminal_with_chase_account(
        chase,
        vec![fill_with_oid(1_001, 42, "100", "1.2")],
        vec![open_order(43, Some(false))],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    assert!(
        terminal
            .toasts
            .iter()
            .any(|toast| toast.is_error && toast.message.contains("over target")),
        "expected an error toast containing the overfill summary"
    );
}
