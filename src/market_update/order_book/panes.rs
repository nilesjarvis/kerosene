use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use std::collections::hash_map::Entry;

impl TradingTerminal {
    pub(crate) fn ensure_order_book_pane_instances(&mut self, fallback_tick_size: f64) {
        let pane_ids = self
            .workspace_pane_kinds()
            .filter_map(|(_, _, kind)| match kind {
                PaneKind::OrderBook(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        for id in pane_ids {
            if let Entry::Vacant(entry) = self.order_books.entry(id) {
                let mut instance = OrderBookInstance::new(
                    id,
                    OrderBookSymbolMode::Active,
                    Self::normalized_book_tick_size(fallback_tick_size),
                );
                instance.book_loading = true;
                entry.insert(instance);
                self.next_order_book_id = self.next_order_book_id.max(id + 1);
            }
        }
    }

    pub(in crate::market_update::order_book) fn add_order_book_pane(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        let workspace = self.add_widget_workspace;
        let Some(focus) = self.add_target_pane_in(workspace) else {
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
                workspace,
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
