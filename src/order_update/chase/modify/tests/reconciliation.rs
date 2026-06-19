use super::fixtures::{
    TEST_ACCOUNT, chase, chase_by_id, empty_ok_exchange_response, exchange_response,
};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseVerificationReason};

#[test]
fn chase_modify_unknown_response_preserves_target_for_reconciliation() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        Err("response body timeout: token=super-secret".to_string()),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert!(terminal.account_loading);
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(!*is_error);
    assert!(message.contains("token=<redacted>"));
    assert!(!message.contains("super-secret"));
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

#[test]
fn chase_modify_malformed_filled_response_preserves_target_for_reconciliation() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.chase_orders.insert(1, chase());

    let response = exchange_response(serde_json::json!({
        "filled": {
            "oid": 42_u64,
            "avgPx": "100"
        }
    }));
    let _task = terminal.handle_chase_modify_result(1, 42, Ok(response));

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 0.0);
    assert_eq!(chase.remaining_size, 1.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(chase.current_oid, Some(42));
    assert!(terminal.account_loading);
}
