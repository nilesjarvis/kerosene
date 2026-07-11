use super::{
    ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason, chase, chase_by_id,
    connected_terminal, exchange_busy_terminal, exchange_ready_terminal,
};
use crate::api::{BookLevel, OrderBook};
use crate::config::ReadDataProvider;
use crate::signing::ChaseStopPhase;

fn source_context(
    terminal: &crate::app_state::TradingTerminal,
    hydromancer_key_generation: Option<u64>,
) -> crate::read_data_provider::MarketDataSourceContext {
    crate::read_data_provider::MarketDataSourceContext {
        hydromancer_key_generation,
        ..terminal.market_data_source_context()
    }
}

#[test]
fn chase_reprice_refreshes_account_before_modifying_resting_order() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.filled_size = 0.1;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert!((chase.remaining_size - 1.0).abs() < 1e-12);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn chase_reprice_after_account_change_stops_and_cancels_original_resting_order() {
    let mut terminal = exchange_ready_terminal();
    terminal.connected_address = Some("0xdef0000000000000000000000000000000000000".to_string());
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
    assert_eq!(
        chase.stop_reason,
        Some((
            "Chase stopped: account changed before reprice".to_string(),
            true
        ))
    );
    assert!(!terminal.account_loading);
    assert!(!terminal.account_reconciliation_required);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error
                    && message.contains("account changed before reprice")
                    && message.contains("cancelling order 42")
            })
    );
}

#[test]
fn chase_reprice_updates_desired_price_while_account_verification_is_pending() {
    let mut terminal = connected_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 102.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(102.0));
    assert_eq!(chase.current_price, 100.0);
}

#[test]
fn chase_reprice_queues_when_exchange_gate_is_busy() {
    let mut terminal = exchange_busy_terminal();
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(chase.reprice_count, 0);
}

#[test]
fn chase_reprice_queues_without_refresh_while_exit_is_pending() {
    let mut terminal = exchange_ready_terminal();
    terminal.config_save_exit_requested = true;
    terminal.chase_orders.insert(1, chase());

    let task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(task.units(), 0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(chase.reprice_count, 0);
    assert!(!terminal.account_loading);
    assert!(!terminal.account_reconciliation_required);
}

#[test]
fn chase_book_lag_clears_queued_reprice_and_verifies_current_order() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_chase_book_lagged(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, None),
        3,
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.desired_price, None);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| !*is_error && message.contains("market data lagged"))
    );
}

#[test]
fn chase_book_lag_ignores_stale_hydromancer_generation() {
    let mut terminal = exchange_ready_terminal();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_chase_book_lagged(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(1)),
        3,
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert!(!terminal.account_loading);

    let _task = terminal.handle_chase_book_lagged(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(2)),
        3,
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.desired_price, None);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert!(terminal.account_loading);
}

#[test]
fn chase_book_lag_gates_provider_source() {
    let mut terminal = exchange_ready_terminal();
    terminal.hydromancer_key_generation = 2;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_chase_book_lagged(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(2)),
        3,
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert!(!terminal.account_loading);

    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    let _task = terminal.handle_chase_book_lagged(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, None),
        3,
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.desired_price, None);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert!(terminal.account_loading);
}

#[test]
fn chase_book_lag_ignores_noncanonical_sigfigs() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_book_lagged(
        1,
        "BTC".to_string(),
        (Some(5), None),
        source_context(&terminal, None),
        3,
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert!(!terminal.account_loading);
}

#[test]
fn chase_reprice_clears_stale_queued_target_when_book_moves_away() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 99.5);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.desired_price, None);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(chase.current_price, 100.0);
}

#[test]
fn chase_book_update_ignores_noncanonical_sigfigs() {
    let mut terminal = exchange_ready_terminal();
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_book_update(
        1,
        "BTC".to_string(),
        (Some(5), None),
        source_context(&terminal, None),
        book(101.0, 102.0),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(chase.desired_price, None);
    assert!(!terminal.account_loading);
}

#[test]
fn chase_book_update_ignores_stale_hydromancer_generation() {
    let mut terminal = exchange_ready_terminal();
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 2;
    terminal.chase_orders.insert(1, chase());

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_chase_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(1)),
        book(101.0, 102.0),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(chase.desired_price, None);
    assert!(!terminal.account_loading);

    let _task = terminal.handle_chase_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        source_context(&terminal, Some(2)),
        book(101.0, 102.0),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert!(terminal.account_loading);
}

#[test]
fn chase_book_update_ignores_stale_hyperliquid_generation() {
    let mut terminal = exchange_ready_terminal();
    terminal.chase_orders.insert(1, chase());
    let stale_context = source_context(&terminal, None);
    terminal.bump_read_data_provider_generation();

    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
    let _task = terminal.handle_chase_book_update(
        1,
        "BTC".to_string(),
        sigfigs,
        stale_context,
        book(101.0, 102.0),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(chase.desired_price, None);
    assert!(!terminal.account_loading);
}

fn book(best_bid: f64, best_ask: f64) -> OrderBook {
    OrderBook {
        bids: vec![BookLevel {
            px: best_bid,
            sz: 1.0,
        }],
        asks: vec![BookLevel {
            px: best_ask,
            sz: 1.0,
        }],
    }
}
