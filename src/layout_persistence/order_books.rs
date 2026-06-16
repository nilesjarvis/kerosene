use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::{OrderBookDisplayMode, OrderBookInstance, OrderBookSymbolMode};
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

// ---------------------------------------------------------------------------
// Layout Order-Book Restoration
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn restore_layout_order_books(
        &mut self,
        layout: &config::SavedLayout,
    ) -> Task<Message> {
        self.order_books.clear();
        self.next_order_book_id = 0;

        for order_book_config in &layout.order_books {
            let mode = match &order_book_config.mode {
                config::OrderBookSymbolModeConfig::Active => OrderBookSymbolMode::Active,
                config::OrderBookSymbolModeConfig::Fixed(symbol) => {
                    if self.symbol_key_is_hidden(symbol) {
                        OrderBookSymbolMode::Active
                    } else {
                        OrderBookSymbolMode::Fixed(symbol.clone())
                    }
                }
            };

            let mut instance = OrderBookInstance::new(
                order_book_config.id,
                mode,
                Self::normalized_order_book_tick_size(
                    order_book_config.tick_size,
                    layout.book_tick_size,
                ),
            );
            instance.display_mode = match order_book_config.display_mode {
                config::OrderBookDisplayModeConfig::DepthList => OrderBookDisplayMode::DepthList,
                config::OrderBookDisplayModeConfig::DomLadder => OrderBookDisplayMode::DomLadder,
                config::OrderBookDisplayModeConfig::DepthChart => OrderBookDisplayMode::DepthChart,
            };
            instance.center_on_mid = order_book_config.center_on_mid;
            instance.reverse_side = order_book_config.reverse_side;
            instance.show_spread_chart = order_book_config.show_spread_chart;
            instance.set_spread_chart_height(order_book_config.spread_chart_height);
            instance.book_loading = true;
            self.order_books.insert(order_book_config.id, instance);
            self.next_order_book_id = self.next_order_book_id.max(order_book_config.id + 1);
        }

        for (_, pane_kind) in self.panes.iter() {
            if let PaneKind::OrderBook(id) = pane_kind
                && !self.order_books.contains_key(id)
            {
                let mut instance = OrderBookInstance::new(
                    *id,
                    OrderBookSymbolMode::Active,
                    Self::normalized_book_tick_size(layout.book_tick_size),
                );
                instance.book_loading = true;
                self.order_books.insert(*id, instance);
                self.next_order_book_id = self.next_order_book_id.max(id + 1);
            }
        }

        self.order_book_fetch_tasks_for_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_tick_size;
    use iced::widget::pane_grid;

    #[test]
    fn restore_layout_normalizes_invalid_fallback_book_tick_size() {
        let (mut terminal, _) = TradingTerminal::boot();
        let mut layout = terminal.saved_layout_snapshot("Legacy".to_string());
        layout.order_books.clear();
        layout.book_tick_size = 0.0;

        let (panes, _) = pane_grid::State::new(PaneKind::OrderBook(42));
        terminal.panes = panes;

        let _task = terminal.restore_layout_order_books(&layout);

        assert_eq!(
            terminal.order_books.get(&42).map(|book| book.tick_size),
            Some(default_tick_size())
        );
    }

    #[test]
    fn restore_layout_normalizes_invalid_per_book_tick_size_to_fallback() {
        let (mut terminal, _) = TradingTerminal::boot();
        let mut layout = terminal.saved_layout_snapshot("Legacy".to_string());
        layout.book_tick_size = 0.25;
        layout.order_books = vec![
            serde_json::from_str(r#"{"id":42}"#)
                .expect("minimal legacy order book config should deserialize"),
        ];

        let _task = terminal.restore_layout_order_books(&layout);

        assert_eq!(
            terminal.order_books.get(&42).map(|book| book.tick_size),
            Some(0.25)
        );
    }
}
