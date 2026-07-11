use super::fixtures::{TEST_ACCOUNT, chase, chase_by_id, exchange_response};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseVerificationReason};

#[test]
fn chase_modify_success_preserves_target_until_account_confirms() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(serde_json::json!("success"))),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(chase.current_oid, Some(42));
    assert_eq!(chase.current_price, 100.0);
    assert_eq!(chase.current_price_wire, "100");
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn chase_modify_success_adopts_resting_oid_from_response() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(
            serde_json::json!({"resting": {"oid": 77}}),
        )),
    );

    // If the exchange ever re-keys an order on modify, the chase must track
    // the oid from the response or reconciliation follows a dead order.
    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.current_oid, Some(77));
    assert!(chase.known_oids.contains(&42));
    assert!(chase.known_oids.contains(&77));
}

#[test]
fn stale_modify_result_from_prior_reprice_does_not_settle_current_reprice() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase();
    chase.reprice_count = 2;
    terminal.chase_orders.insert(1, chase);
    terminal.order_status = Some(("Current reprice pending".to_string(), false));

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(serde_json::json!("success"))),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
    assert_eq!(chase.reprice_count, 2);
    assert_eq!(chase.current_oid, Some(42));
    assert_eq!(chase.current_price, 100.0);
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(
        terminal.order_status,
        Some(("Current reprice pending".to_string(), false))
    );
}

#[test]
fn duplicate_modify_result_cannot_rewrite_settled_reprice() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(serde_json::json!("success"))),
    );
    let settled_status = terminal.order_status.clone();

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(serde_json::json!({
            "error": "conflicting duplicate"
        }))),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert_eq!(chase.current_oid, Some(42));
    assert_eq!(terminal.order_status, settled_status);
}
