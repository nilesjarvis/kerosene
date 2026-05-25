use super::*;
use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseOrder, ChaseStopPhase, ChaseVerificationReason};
use std::time::Instant;

mod oid_status;
mod placement_status;
mod stop_cancel;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
const TEST_CLOID: &str = "0x1234567890abcdef1234567890abcdef";

fn chase() -> ChaseOrder {
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
    match serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": statuses
            }
        }
    })) {
        Ok(response) => response,
        Err(e) => panic!("test exchange response should deserialize: {e}"),
    }
}

fn open_order_status() -> OrderStatusResult {
    OrderStatusResult {
        status: "open".to_string(),
        oid: Some(9001),
        cloid: Some(TEST_CLOID.to_string()),
        raw_summary: "open (oid 9001, cloid 0x1234567890abcdef1234567890abcdef)".to_string(),
    }
}

fn oid_status(status: &str) -> OrderStatusResult {
    OrderStatusResult {
        status: status.to_string(),
        oid: Some(9001),
        cloid: None,
        raw_summary: format!("{status} (oid 9001)"),
    }
}

fn terminal_with_chase(chase: ChaseOrder) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.chase_orders.insert(chase.id, chase);
    terminal
}

fn connected_terminal_with_chase(chase: ChaseOrder) -> TradingTerminal {
    let mut terminal = terminal_with_chase(chase);
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal
}

fn chase_from_terminal(terminal: &TradingTerminal, chase_id: u64) -> &ChaseOrder {
    match terminal.chase_orders.get(&chase_id) {
        Some(chase) => chase,
        None => panic!("chase {chase_id} should remain"),
    }
}

fn order_status_is_error_containing(terminal: &TradingTerminal, needle: &str) -> bool {
    terminal
        .order_status
        .as_ref()
        .is_some_and(|(message, is_error)| *is_error && message.contains(needle))
}
