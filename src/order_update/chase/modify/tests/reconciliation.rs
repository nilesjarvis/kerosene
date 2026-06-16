use super::fixtures::{TEST_ACCOUNT, chase, chase_by_id, empty_ok_exchange_response};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseVerificationReason};

#[test]
fn chase_modify_unknown_response_preserves_target_for_reconciliation() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.chase_orders.insert(1, chase());

    let _task =
        terminal.handle_chase_modify_result(1, 42, Err("response body timeout".to_string()));

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert!(terminal.account_loading);
}

#[test]
fn chase_modify_empty_ok_response_preserves_target_for_reconciliation() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(1, 42, Ok(empty_ok_exchange_response()));

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(chase.current_price, 100.0);
    assert_eq!(chase.current_oid, Some(42));
    assert!(terminal.account_loading);
}
