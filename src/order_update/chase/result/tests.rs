use super::*;
use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseOrder, ChaseStopPhase, ChaseVerificationReason};
use std::time::Instant;

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
        known_oids: Vec::new(),
        current_cloid: None,
        place_attempt_count: 0,
        asset: 7,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        current_oid: None,
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at,
        started_at_ms: 1_000,
        reprice_count: 0,
        lifecycle: ChaseLifecycle::LoadingBook,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

fn exchange_response(statuses: Vec<serde_json::Value>) -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": statuses
            }
        }
    }))
    .expect("test exchange response should deserialize")
}

#[test]
fn stopped_chase_place_result_requests_cancel_for_late_resting_order() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::AwaitingPlace,
    };
    let response = exchange_response(vec![serde_json::json!({
        "resting": {
            "oid": 9001_u64
        }
    })]);

    assert_eq!(
        stopped_chase_cancel_request(&chase, &response),
        Some(StoppedChaseCancelRequest {
            chase_id: 1,
            agent_key: "agent-key".to_string().into(),
            asset: 7,
            oid: 9001
        })
    );
}

#[test]
fn active_chase_place_result_does_not_request_stop_cancel() {
    let chase = chase();
    let response = exchange_response(vec![serde_json::json!({
        "resting": {
            "oid": 9001_u64
        }
    })]);

    assert_eq!(stopped_chase_cancel_request(&chase, &response), None);
}

#[test]
fn chase_place_status_open_recovers_oid_after_unknown_place_response() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Placing;
    chase.current_cloid = Some("0x1234567890abcdef1234567890abcdef".to_string());
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        "0x1234567890abcdef1234567890abcdef".to_string(),
        Ok(OrderStatusResult {
            status: "open".to_string(),
            oid: Some(9001),
            cloid: Some("0x1234567890abcdef1234567890abcdef".to_string()),
            raw_summary: "open (oid 9001, cloid 0x1234567890abcdef1234567890abcdef)".to_string(),
        }),
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.current_oid, Some(9001));
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement
        }
    );
    assert!(chase.known_oids.contains(&9001));
}

#[test]
fn chase_place_status_error_keeps_chase_uncertain_for_retry() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Placement,
    };
    chase.current_cloid = Some("0x1234567890abcdef1234567890abcdef".to_string());
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        "0x1234567890abcdef1234567890abcdef".to_string(),
        Err("status endpoint down".to_string()),
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("placement status still uncertain")
            })
    );
}

#[test]
fn chase_oid_status_error_keeps_chase_uncertain_for_reconciliation() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Modify,
    };
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_order_oid_status_result(
        1,
        9001,
        Err("status endpoint down".to_string()),
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("order status still uncertain")
            })
    );
}

#[test]
fn chase_oid_status_canceled_stops_without_authorizing_replacement() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.desired_price = Some(101.0);
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::MissingOrder,
    };
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_order_oid_status_result(
        1,
        9001,
        Ok(OrderStatusResult {
            status: "canceled".to_string(),
            oid: Some(9001),
            cloid: None,
            raw_summary: "canceled (oid 9001)".to_string(),
        }),
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.desired_price, None);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::VerifyingCancel { oid: 9001 }
        }
    );
}

#[test]
fn chase_oid_status_rejected_authorizes_replacement_after_refresh() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.desired_price = Some(101.0);
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::MissingOrder,
    };
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_order_oid_status_result(
        1,
        9001,
        Ok(OrderStatusResult {
            status: "rejected".to_string(),
            oid: Some(9001),
            cloid: None,
            raw_summary: "rejected (oid 9001)".to_string(),
        }),
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrderResolvedNoFill
        }
    );
}
