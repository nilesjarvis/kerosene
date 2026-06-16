use super::*;

#[test]
fn start_twap_keeps_base_size_when_quantity_is_not_usd() {
    let mut terminal = twap_ready_terminal();

    let _task = terminal.start_twap(true);

    let twap = started_twap_or_panic(&terminal);
    assert_eq!(twap.target_size, 2.5);
    assert_eq!(twap.slice_count, 2);
    assert_eq!(twap.min_price, 90.0);
    assert_eq!(twap.max_price, 110.0);
    assert_eq!(twap.status, TwapStatus::WaitingForMarket);
}

#[test]
fn start_twap_from_snapshot_accepts_matching_click_context() {
    let mut terminal = twap_ready_terminal();
    let snapshot = terminal.twap_order_start_snapshot();

    let _task = terminal.start_twap_from_snapshot(true, snapshot);

    let twap = started_twap_or_panic(&terminal);
    assert_eq!(twap.target_size, 2.5);
    assert_eq!(twap.slice_count, 2);
    assert!(twap.is_buy);
}

#[test]
fn start_twap_from_snapshot_rejects_changed_click_context() {
    let mut terminal = twap_ready_terminal();
    let snapshot = terminal.twap_order_start_snapshot();
    terminal.twap_form.slices = "3".to_string();

    let _task = terminal.start_twap_from_snapshot(true, snapshot);

    assert!(terminal.twap_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("TWAP settings changed")
            }),
        "status should explain the stale click context: {:?}",
        terminal.order_status
    );
}

#[test]
fn start_twap_rejects_duplicate_start_within_window() {
    let mut terminal = twap_ready_terminal();

    let _task = terminal.start_twap(true);
    assert_eq!(terminal.twap_orders.len(), 1);

    // start_twap is synchronous, so a queued double click replays it
    // immediately; the duplicate-start window must absorb the second press.
    let _task = terminal.start_twap(true);

    assert_eq!(terminal.twap_orders.len(), 1);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| *is_error && message.contains("just started"))
    );
}

#[test]
fn start_twap_allows_opposite_side_despite_recent_start() {
    let mut terminal = twap_ready_terminal();

    let _task = terminal.start_twap(true);
    let _task = terminal.start_twap(false);

    assert_eq!(terminal.twap_orders.len(), 2);
}

#[test]
fn start_twap_rejects_usd_notional_without_fresh_mid() {
    let mut terminal = twap_ready_terminal();
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;

    let _task = terminal.start_twap(true);

    assert!(terminal.twap_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("Cannot start USD TWAP: no fresh mid price")
            })
    );
}

#[test]
fn start_twap_rejects_non_orderable_fallback_outcome() {
    let mut terminal = twap_ready_terminal();
    terminal.active_symbol = "#66".to_string();
    terminal.active_symbol_display = "Recurring Fallback".to_string();
    terminal.exchange_symbols = vec![fallback_outcome_symbol("#66")];

    let _task = terminal.start_twap(true);

    assert!(terminal.twap_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        order_status_error_contains(&terminal, "not a tradable market"),
        "status should explain the non-orderable symbol: {:?}",
        terminal.order_status
    );
}
