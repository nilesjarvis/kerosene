use super::*;

#[test]
fn outcome_symbols_are_available_for_order_book_fetches() {
    let terminal = TradingTerminal::boot().0;

    assert_eq!(terminal.order_book_unavailable_reason("#650"), None);
}
