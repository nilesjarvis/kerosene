use crate::app_state::TradingTerminal;
use crate::market_state::{OrderBookId, OrderBookSymbolMode};

// ---------------------------------------------------------------------------
// Symbol Availability
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn order_book_symbol_for_mode(&self, mode: &OrderBookSymbolMode) -> String {
        match mode {
            OrderBookSymbolMode::Active => self.active_symbol.clone(),
            OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
        }
    }

    pub(crate) fn order_book_unavailable_reason(&self, symbol: &str) -> Option<String> {
        if symbol.is_empty() {
            return Some("No order-book symbol selected".to_string());
        }
        if self.symbol_key_is_hidden(symbol) {
            return Some("Order book ticker is hidden in Settings > Risk".to_string());
        }
        None
    }

    pub(in crate::market_update::order_book) fn order_book_instance_is_muted(
        &self,
        id: OrderBookId,
    ) -> bool {
        self.order_books.get(&id).is_some_and(|inst| {
            let symbol = match &inst.mode {
                OrderBookSymbolMode::Active => self.active_symbol.clone(),
                OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
            };
            self.symbol_key_is_hidden(&symbol)
        })
    }
}
