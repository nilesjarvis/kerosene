use super::*;
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseOrder, ChasePendingOp};
use std::time::{Duration, Instant};

fn chase() -> ChaseOrder {
    let started_at = Instant::now();
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at,
        started_at_ms: 1_000,
        reprice_count: 1,
        pending_op: Some(ChasePendingOp::Modify { oid: 42 }),
        last_reprice_at: Some(started_at),
        pending_best_price: Some(101.0),
        pending_size_correction: false,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: true,
        missing_open_order_refresh_requested: false,
    }
}

fn exchange_response(status: serde_json::Value) -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": [status]
            }
        }
    }))
    .expect("test exchange response should deserialize")
}

#[test]
fn chase_modify_success_updates_expected_price_without_refreshing_account() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        Ok(exchange_response(serde_json::json!("success"))),
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, None);
    assert_eq!(chase.pending_best_price, None);
    assert!(!chase.pending_size_correction);
    assert_eq!(chase.current_oid, Some(42));
    assert_eq!(chase.current_price, 101.0);
    assert_eq!(chase.current_price_wire, "101");
    assert!(!chase.oid_confirmed);
    assert!(!terminal.account_loading);
}

#[test]
fn chase_modify_rate_limit_keeps_target_queued_for_retry() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        Ok(exchange_response(serde_json::json!({
            "error": "rate limit"
        }))),
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, None);
    assert_eq!(chase.pending_best_price, Some(101.0));
    assert_eq!(chase.current_price, 100.0);
    assert!(!chase.can_reprice_now(Instant::now() + Duration::from_secs(4)));
    assert!(terminal
        .last_advanced_exchange_request_at
        .is_some_and(|last| last > Instant::now()));

    let (status, is_error) = terminal.order_status.as_ref().expect("status");
    assert!(*is_error);
    assert!(status.contains("rate limit"));
}

#[test]
fn chase_modify_rate_limit_keeps_size_correction_queued_for_retry() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase();
    chase.pending_size_correction = true;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        Ok(exchange_response(serde_json::json!({
            "error": "too many requests"
        }))),
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, None);
    assert!(chase.pending_size_correction);
    assert_eq!(chase.pending_best_price, Some(101.0));
}

#[test]
fn chase_modify_unknown_response_preserves_target_for_reconciliation() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.chase_orders.insert(1, chase());

    let _task =
        terminal.handle_chase_modify_result(1, 42, Err("response body timeout".to_string()));

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, None);
    assert_eq!(chase.pending_best_price, Some(101.0));
    assert!(chase.missing_open_order_refresh_requested);
    assert!(terminal.account_loading);
}
