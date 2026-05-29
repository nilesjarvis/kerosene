use crate::app_state::TradingTerminal;
use crate::market_state::OrderBookId;
use crate::message::Message;
use crate::signing::OrderKind;
use iced::Task;

mod selection;

use selection::{OrderBookPriceSelectionError, order_book_price_selection};

// ---------------------------------------------------------------------------
// Order Book Price Selection
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_order_book_price_selected(
        &mut self,
        id: OrderBookId,
        price: String,
    ) -> Task<Message> {
        let selection = match order_book_price_selection(
            self.order_books.get(&id),
            &self.active_symbol,
            &price,
        ) {
            Ok(selection) => selection,
            Err(OrderBookPriceSelectionError::InvalidPrice) => {
                self.order_status = Some(("Invalid order-book price".into(), true));
                return Task::none();
            }
            Err(OrderBookPriceSelectionError::Unavailable) => {
                self.order_status = Some(("Order book unavailable".into(), true));
                return Task::none();
            }
        };

        let mut task = Task::none();
        if selection.target_symbol != self.active_symbol {
            let previous_symbol = self.active_symbol.clone();
            task = self.switch_active_symbol_internal(selection.target_symbol);
            if self.active_symbol == previous_symbol {
                return task;
            }
        }

        if let Some(mid) = selection.book_mid {
            self.all_mids.insert(self.active_symbol.clone(), mid);
            self.all_mids_updated_at_ms
                .insert(self.active_symbol.clone(), Self::now_ms());
            self.sync_chart_market_reference_prices();
        }

        self.order_kind = OrderKind::Limit;
        self.order_price = selection.selected_price;
        self.persist_config();
        task
    }
}
