use super::fixtures::{account_data_with_timestamp, chase_order, chase_order_by_id, open_order};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseOrder, ChaseVerificationReason};
use crate::ws::WsUserData;

mod account_refresh;
mod websocket;

const CONNECTED_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

fn connected_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(CONNECTED_ADDRESS.to_string());
    terminal
}

fn refresh_ready_terminal() -> TradingTerminal {
    let mut terminal = connected_terminal();
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    terminal
}

fn verifying_chase(reason: ChaseVerificationReason) -> ChaseOrder {
    let mut chase = chase_order();
    chase.lifecycle = ChaseLifecycle::Verifying { reason };
    chase.desired_price = Some(101.0);
    chase
}

fn reprice_verification_chase() -> ChaseOrder {
    let mut chase = verifying_chase(ChaseVerificationReason::Reprice);
    chase.current_price = 101.0;
    chase.current_price_wire = "101".to_string();
    chase
}
