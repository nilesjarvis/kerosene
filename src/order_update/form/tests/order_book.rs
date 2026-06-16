use super::{
    OrderBookSymbolMode, TradingTerminal, book, set_order_book, symbol, terminal_with_order_book,
};
use crate::api::MarketType;
use crate::app_state::sensitive_string;
use crate::config::AccountProfile;
use crate::order_execution::PendingOrderAction;

mod selection;
mod submission;

fn terminal_ready_for_order_book_submission(
    quantity: &str,
    quantity_is_usd: bool,
) -> TradingTerminal {
    let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);
    let account = "0xabc0000000000000000000000000000000000000";
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.connected_address = Some(account.to_string());
    terminal.wallet_address_input = account.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: account.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
    terminal.set_committed_agent_key_for_test("agent-key");
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
