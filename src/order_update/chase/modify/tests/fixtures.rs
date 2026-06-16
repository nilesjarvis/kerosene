use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseOrder, ExchangeResponse};

use std::time::Instant;

pub(super) const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

pub(super) fn chase() -> ChaseOrder {
    let started_at = Instant::now();
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: TEST_ACCOUNT.to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        current_cloid: None,
        place_attempt_count: 0,
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
        fill_cutoff_ms_by_oid: Vec::new(),
        reprice_count: 1,
        lifecycle: ChaseLifecycle::Modifying { oid: 42 },
        last_reprice_at: Some(started_at),
        desired_price: Some(101.0),
        stop_reason: None,
        cancel_retries: 0,
    }
}

pub(super) fn exchange_response(status: serde_json::Value) -> ExchangeResponse {
    exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [status]
                }
            }
        }),
        "test exchange response should deserialize",
    )
}

pub(super) fn exchange_response_from_value(
    value: serde_json::Value,
    context: &str,
) -> ExchangeResponse {
    match serde_json::from_value(value) {
        Ok(response) => response,
        Err(error) => panic!("{context}: {error}"),
    }
}

pub(super) fn empty_ok_exchange_response() -> ExchangeResponse {
    exchange_response_from_value(
        serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": []
            }
        }
        }),
        "empty ok exchange response should deserialize",
    )
}

pub(super) fn chase_by_id(terminal: &TradingTerminal, chase_id: u64) -> &ChaseOrder {
    match terminal.chase_orders.get(&chase_id) {
        Some(chase) => chase,
        None => panic!("chase should remain"),
    }
}

pub(super) fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match terminal.order_status.as_ref() {
        Some((status, is_error)) => (status.as_str(), *is_error),
        None => panic!("status"),
    }
}
