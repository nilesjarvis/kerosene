use super::*;

#[test]
fn order_book_price_selected_seeds_mid_for_immediate_limit_submission() {
    let mut terminal = terminal_ready_for_order_book_submission("1", false);

    let _task = terminal.handle_order_book_price_selected(7, "100".to_string());
    let _task = terminal.execute_order(true);

    assert_pending_buy_submission(&terminal);
}

#[test]
fn order_book_price_selected_allows_immediate_usd_limit_submission() {
    let mut terminal = terminal_ready_for_order_book_submission("100", true);

    let _task = terminal.handle_order_book_price_selected(7, "100".to_string());
    let _task = terminal.execute_order(true);

    assert_pending_buy_submission(&terminal);
}
