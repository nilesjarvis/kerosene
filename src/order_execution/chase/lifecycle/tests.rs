use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseOrder};

use std::time::Instant;

mod context;
mod limits;
mod place;
mod reprice;
mod stop;

fn chase() -> ChaseOrder {
    let started_at = Instant::now();
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "original-agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at,
        started_at_ms: 1_000,
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Resting,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

fn chase_by_id(terminal: &TradingTerminal, id: u64) -> &ChaseOrder {
    match terminal.chase_orders.get(&id) {
        Some(chase) => chase,
        None => panic!("chase should remain"),
    }
}
