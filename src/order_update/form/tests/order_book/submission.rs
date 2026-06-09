use super::*;

#[test]
fn order_book_price_selected_does_not_seed_mid_for_immediate_limit_submission() {
    let mut terminal = terminal_ready_for_order_book_submission("1", false);

    let _task = terminal.handle_order_book_price_selected(7, "100".to_string());
    let _task = terminal.execute_order(true);

    assert!(terminal.pending_order_action.is_none());
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.starts_with("No mid price for BTC")
            })
    );
    assert!(!terminal.all_mids.contains_key("BTC"));
}

#[test]
fn order_book_price_selected_uses_existing_live_mid_for_immediate_usd_limit_submission() {
    let mut terminal = terminal_ready_for_order_book_submission("100", true);
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let _task = terminal.handle_order_book_price_selected(7, "100".to_string());
    let _task = terminal.execute_order(true);

    assert_pending_buy_submission(&terminal);
}
