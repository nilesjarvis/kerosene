use super::fixtures::{
    empty_account_data, filled_status, pending_twap, reconciliation_deadline, test_twap,
    twap_by_id, user_fill,
};
use crate::app_state::TradingTerminal;
use crate::twap_state::{TWAP_RECONCILIATION_TIMEOUT, TwapChildStatus, TwapOrder, TwapStatus};

use std::time::Instant;

mod origin;
mod status_check;
mod timeout;

const CLOID: &str = "0x1234567890abcdef1234567890abcdef";
const ORIGIN_ADDRESS: &str = "0xabc";
const SWITCHED_ADDRESS: &str = "0xdef";
const CHILD_OID: u64 = 42;

fn terminal_for_account(address: &str) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(address.to_string());
    terminal
}

fn switched_account_terminal() -> TradingTerminal {
    terminal_for_account(SWITCHED_ADDRESS)
}

fn origin_account_terminal() -> TradingTerminal {
    terminal_for_account(ORIGIN_ADDRESS)
}

fn reconciliation_twap(now: Instant) -> TwapOrder {
    let mut twap = test_twap(1, CLOID, now);
    twap.child_orders[0].oid = Some(CHILD_OID);
    twap.child_orders[0].status = TwapChildStatus::AwaitingReconciliation;
    twap.status_check_cloid = Some(CLOID.to_string());
    twap.reconciliation_deadline = Some(now + TWAP_RECONCILIATION_TIMEOUT);
    twap
}

fn disable_current_account_refresh(terminal: &mut TradingTerminal) {
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
}
