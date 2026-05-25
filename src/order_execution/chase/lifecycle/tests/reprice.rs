use super::{chase, chase_by_id};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason};

use std::time::{Duration, Instant};

mod direct;
mod reconciliation;
mod tick;

const CONNECTED_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

fn connected_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(CONNECTED_ADDRESS.to_string());
    terminal
}

fn exchange_ready_terminal() -> TradingTerminal {
    let mut terminal = connected_terminal();
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    terminal
}

fn exchange_busy_terminal() -> TradingTerminal {
    let mut terminal = exchange_ready_terminal();
    terminal.last_advanced_exchange_request_at = Some(Instant::now());
    terminal
}
