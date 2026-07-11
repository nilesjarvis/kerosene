use super::fixtures::{
    canceled_status, empty_account_data, filled_status, missing_status, open_status, pending_twap,
    reconciliation_deadline, rejected_status, test_twap, twap_by_id, user_fill,
};
use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::twap_state::{
    TWAP_MAX_RETRY_ATTEMPTS, TWAP_RECONCILIATION_TIMEOUT, TwapChildStatus, TwapOrder, TwapStatus,
};

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

fn set_account_data_for_connected_account(terminal: &mut TradingTerminal, data: AccountData) {
    terminal.account_data_address = terminal.connected_address.clone();
    terminal.account_data = Some(data);
}

fn reconciliation_twap(now: Instant) -> TwapOrder {
    let mut twap = test_twap(1, CLOID, now);
    twap.child_orders[0].oid = Some(CHILD_OID);
    twap.child_orders[0].status = TwapChildStatus::AwaitingReconciliation;
    twap.status_check_cloid = Some(CLOID.to_string());
    twap.status_check_pending_attempt = None;
    twap.reconciliation_deadline = Some(now + TWAP_RECONCILIATION_TIMEOUT);
    twap
}

fn disable_current_account_refresh(terminal: &mut TradingTerminal) {
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
}
