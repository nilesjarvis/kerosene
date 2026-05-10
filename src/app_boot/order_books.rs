use crate::app_state::TradingTerminal;
use crate::config::{self, KeroseneConfig};
use crate::market_state::{OrderBookDisplayMode, OrderBookInstance, OrderBookSymbolMode};
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Task;
use std::collections::HashSet;

impl TradingTerminal {
    pub(super) fn boot_order_book_instances(
        &mut self,
        cfg: &KeroseneConfig,
        muted_tickers: &HashSet<String>,
    ) {
        for ob_cfg in &cfg.order_books {
            let mode = match &ob_cfg.mode {
                config::OrderBookSymbolModeConfig::Active => OrderBookSymbolMode::Active,
                config::OrderBookSymbolModeConfig::Fixed(s) => {
                    if Self::key_matches_muted_tickers(&[], muted_tickers, s) {
                        OrderBookSymbolMode::Active
                    } else {
                        OrderBookSymbolMode::Fixed(s.clone())
                    }
                }
            };
            let mut inst = OrderBookInstance::new(ob_cfg.id, mode, ob_cfg.tick_size);
            inst.display_mode = match ob_cfg.display_mode {
                config::OrderBookDisplayModeConfig::DepthList => OrderBookDisplayMode::DepthList,
                config::OrderBookDisplayModeConfig::DomLadder => OrderBookDisplayMode::DomLadder,
            };
            inst.show_spread_chart = ob_cfg.show_spread_chart;
            inst.spread_chart_height = ob_cfg.spread_chart_height;
            inst.book_loading = true;
            self.order_books.insert(ob_cfg.id, inst);
            self.next_order_book_id = self.next_order_book_id.max(ob_cfg.id + 1);
        }

        for (_, pane_cfg) in self.panes.iter() {
            if let PaneKind::OrderBook(id) = pane_cfg
                && !self.order_books.contains_key(id)
            {
                let mut inst =
                    OrderBookInstance::new(*id, OrderBookSymbolMode::Active, cfg.book_tick_size);
                inst.book_loading = true;
                self.order_books.insert(*id, inst);
                self.next_order_book_id = self.next_order_book_id.max(id + 1);
            }
        }
    }

    pub(super) fn boot_order_book_tasks(&mut self) -> Task<Message> {
        self.order_book_fetch_tasks_for_all()
    }
}
