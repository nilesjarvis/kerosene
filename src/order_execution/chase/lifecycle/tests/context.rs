use super::super::chase_account_matches;
use super::chase;
use crate::app_state::TradingTerminal;

use std::time::Instant;

#[test]
fn chase_context_allows_same_connected_account() {
    assert!(chase_account_matches(
        &chase(),
        Some("0xabc0000000000000000000000000000000000000")
    ));
}

#[test]
fn chase_context_rejects_changed_or_disconnected_account() {
    assert!(!chase_account_matches(
        &chase(),
        Some("0xdef0000000000000000000000000000000000000")
    ));
    assert!(!chase_account_matches(&chase(), None));
}

#[test]
fn chase_exchange_requests_pause_while_account_reconciliation_is_loading() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;

    assert!(terminal.can_send_chase_exchange_request(now));

    terminal.account_loading = true;

    assert!(!terminal.can_send_chase_exchange_request(now));

    terminal.account_loading = false;
    terminal.account_reconciliation_required = true;

    assert!(!terminal.can_send_chase_exchange_request(now));
}
