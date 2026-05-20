use super::*;
use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::signing::ChaseOrder;
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
        pending_op: None,
        last_reprice_at: None,
        pending_best_price: None,
        pending_size_correction: false,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: false,
        missing_open_order_refresh_requested: false,
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
    chase.stop_requested = true;
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
    chase.pending_op = Some(ChasePendingOp::Place);
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
    assert_eq!(chase.pending_op, None);
    assert!(chase.known_oids.contains(&9001));
    assert!(chase.missing_open_order_refresh_requested);
}

#[test]
fn chase_place_status_error_archives_failed_chase_as_error() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase();
    chase.pending_op = Some(ChasePendingOp::Place);
    chase.current_cloid = Some("0x1234567890abcdef1234567890abcdef".to_string());
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        "0x1234567890abcdef1234567890abcdef".to_string(),
        Err("status endpoint down".to_string()),
    );

    assert!(terminal.chase_orders.is_empty());
    assert!(
        terminal
            .advanced_order_history
            .front()
            .is_some_and(|entry| {
                entry.status == "Error"
                    && entry.summary.contains("could not confirm placement status")
            })
    );
}
