use super::fixtures::{test_twap, twap_by_id};
use crate::app_state::TradingTerminal;
use crate::twap_state::{TwapChildStatus, TwapStatus};

use std::time::Duration;
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

#[test]
fn twap_deadline_waits_for_pending_status_reconciliation() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut status_check_twap = test_twap(1, "0xaaa", now);
    status_check_twap.ends_at = now - Duration::from_secs(1);
    status_check_twap.child_orders[0].status = TwapChildStatus::NoFill;
    terminal.twap_orders.insert(1, status_check_twap);

    assert!(!terminal.expire_twap_if_deadline_passed(1, now));
    assert_eq!(twap_by_id(&terminal, 1).status, TwapStatus::Paused);

    let mut unknown_child_twap = test_twap(2, "0xbbb", now);
    unknown_child_twap.ends_at = now - Duration::from_secs(1);
    unknown_child_twap.status_check_cloid = None;
    unknown_child_twap.child_orders[0].status = TwapChildStatus::StatusUnknown;
    terminal.twap_orders.insert(2, unknown_child_twap);

    assert!(!terminal.expire_twap_if_deadline_passed(2, now));
    assert_eq!(twap_by_id(&terminal, 2).status, TwapStatus::Paused);
}
