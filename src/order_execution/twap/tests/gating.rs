use crate::app_state::TradingTerminal;

use std::time::Instant;

#[test]
fn advanced_exchange_requests_pause_while_account_reconciliation_is_loading() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;

    assert!(terminal.can_send_advanced_exchange_request(now));

    terminal.account_loading = true;

    assert!(!terminal.can_send_advanced_exchange_request(now));

    terminal.account_loading = false;
    terminal.account_reconciliation_required = true;

    assert!(!terminal.can_send_advanced_exchange_request(now));
}
