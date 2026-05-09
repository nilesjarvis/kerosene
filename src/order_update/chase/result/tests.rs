use super::*;
use crate::signing::ChaseOrder;
use std::time::Instant;

fn chase() -> ChaseOrder {
    let started_at = Instant::now();
    ChaseOrder {
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        remaining_size: 1.0,
        asset: 7,
        sz_decimals: 3,
        reduce_only: false,
        current_oid: None,
        current_price: 100.0,
        initial_price: 100.0,
        started_at,
        reprice_count: 0,
        cancel_in_flight: false,
        stop_requested: false,
        cancel_retries: 0,
        oid_confirmed: false,
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
