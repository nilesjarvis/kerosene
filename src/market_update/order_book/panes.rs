use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

impl TradingTerminal {
    pub(in crate::market_update::order_book) fn add_order_book_pane(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        let Some(focus) = self.add_target_pane() else {
            self.push_toast(
                "Could not add Order Book: no pane is available".to_string(),
                true,
            );
            return Task::none();
        };

        let id = self.next_order_book_id;
        self.next_order_book_id += 1;

        let mid = self.resolve_mid_for_symbol(&self.active_symbol);
        let tick = mid.map(helpers::default_tick_for_price).unwrap_or(0.01);
        let mut instance = OrderBookInstance::new(id, OrderBookSymbolMode::Active, tick);
        instance.book_loading = true;
        instance.book_error = None;
        self.order_books.insert(id, instance);

        if self
            .add_pane_to_target(
                self.add_widget_axis(),
                focus,
                PaneKind::OrderBook(id),
                "Order Book",
            )
            .is_none()
        {
            self.order_books.remove(&id);
            return Task::none();
        }

        self.order_book_fetch_task_for_id(id)
    }
}
