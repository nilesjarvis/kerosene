use super::{
    OrderBookSymbolMode, TradingTerminal, book, set_order_book, symbol, terminal_with_order_book,
};
use crate::api::MarketType;
use crate::order_execution::PendingOrderAction;

mod selection;
mod submission;

fn terminal_ready_for_order_book_submission(
    quantity: &str,
    quantity_is_usd: bool,
) -> TradingTerminal {
    let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.connected_address = Some("0xabc".to_string());
    terminal.wallet_key_input = "agent-key".to_string().into();
    terminal.order_quantity = quantity.to_string();
    terminal.order_quantity_is_usd = quantity_is_usd;
    set_order_book(&mut terminal, 7, book(99.0, 101.0));
    terminal
}

fn assert_pending_buy_submission(terminal: &TradingTerminal) {
    assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
    assert_eq!(
        terminal.order_status,
        Some(("Placing order...".to_string(), false))
    );
}
