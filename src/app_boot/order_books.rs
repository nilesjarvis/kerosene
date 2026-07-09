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
            let mut inst = OrderBookInstance::new(
                ob_cfg.id,
                mode,
                Self::normalized_order_book_tick_size(ob_cfg.tick_size, cfg.book_tick_size),
            );
            inst.display_mode = match ob_cfg.display_mode {
                config::OrderBookDisplayModeConfig::DepthList => OrderBookDisplayMode::DepthList,
                config::OrderBookDisplayModeConfig::DomLadder => OrderBookDisplayMode::DomLadder,
                config::OrderBookDisplayModeConfig::DepthChart => OrderBookDisplayMode::DepthChart,
            };
            inst.center_on_mid = ob_cfg.center_on_mid;
            inst.reverse_side = ob_cfg.reverse_side;
            inst.show_spread_chart = ob_cfg.show_spread_chart;
            inst.set_spread_chart_height(ob_cfg.spread_chart_height);
            inst.book_loading = true;
            self.order_books.insert(ob_cfg.id, inst);
            self.next_order_book_id = self.next_order_book_id.max(ob_cfg.id + 1);
        }

        for (_, pane_cfg) in self.panes.iter() {
            if let PaneKind::OrderBook(id) = pane_cfg
                && !self.order_books.contains_key(id)
            {
                let mut inst = OrderBookInstance::new(
                    *id,
                    OrderBookSymbolMode::Active,
                    Self::normalized_book_tick_size(cfg.book_tick_size),
                );
                inst.book_loading = true;
                self.order_books.insert(*id, inst);
                self.next_order_book_id = self.next_order_book_id.max(id + 1);
            }
        }
    }

    pub(super) fn boot_order_book_tasks(&mut self) -> Task<Message> {
        // `@0` is the legacy persisted key for the API-named PURR/USDC pair.
        // Defer it until strict spot metadata rewrites the instance; sending
        // raw `@0` to l2Book fails and can start a noisy retry streak.
        let ids: Vec<_> = self
            .order_books
            .iter()
            .filter_map(|(id, instance)| {
                (self.order_book_symbol_for_mode(&instance.mode) != "@0").then_some(*id)
            })
            .collect();
        Task::batch(
            ids.into_iter()
                .map(|id| self.order_book_fetch_task_for_id(id)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KeroseneConfig, OrderBookConfig, default_tick_size};

    #[test]
    fn boot_normalizes_invalid_fallback_book_tick_size() {
        let mut cfg = KeroseneConfig {
            book_tick_size: 0.0,
            order_books: Vec::new(),
            pane_layout: None,
            ..KeroseneConfig::default()
        };

        let (terminal, _) = TradingTerminal::boot_from_config(cfg.clone());

        assert_eq!(
            terminal.order_books.get(&0).map(|book| book.tick_size),
            Some(default_tick_size())
        );
        TradingTerminal::register_last_layout(&mut cfg);
        assert_eq!(
            cfg.saved_layouts
                .iter()
                .find(|layout| layout.name == "last")
                .map(|layout| layout.book_tick_size),
            Some(default_tick_size())
        );
    }

    #[test]
    fn boot_normalizes_invalid_per_book_tick_size_to_fallback() {
        let cfg = KeroseneConfig {
            book_tick_size: 0.5,
            order_books: vec![
                serde_json::from_str::<OrderBookConfig>(r#"{"id":0}"#)
                    .expect("minimal legacy order book config should deserialize"),
            ],
            pane_layout: None,
            ..KeroseneConfig::default()
        };

        let (terminal, _) = TradingTerminal::boot_from_config(cfg);

        assert_eq!(
            terminal.order_books.get(&0).map(|book| book.tick_size),
            Some(0.5)
        );
    }

    #[test]
    fn boot_keeps_valid_per_book_tick_size_over_invalid_fallback() {
        let order_book = serde_json::from_str::<OrderBookConfig>(r#"{"id":0,"tick_size":2.5}"#)
            .expect("order book config should deserialize");
        let cfg = KeroseneConfig {
            book_tick_size: 0.0,
            order_books: vec![order_book],
            pane_layout: None,
            ..KeroseneConfig::default()
        };

        let (terminal, _) = TradingTerminal::boot_from_config(cfg);

        assert_eq!(
            terminal.order_books.get(&0).map(|book| book.tick_size),
            Some(2.5)
        );
    }

    #[test]
    fn boot_defers_legacy_api_named_spot_book_until_metadata_migration() {
        let order_book = serde_json::from_str::<OrderBookConfig>(
            r#"{"id":7,"mode":{"Fixed":"@0"},"tick_size":0.01}"#,
        )
        .expect("legacy fixed spot order book config");
        let cfg = KeroseneConfig {
            order_books: vec![order_book],
            pane_layout: None,
            ..KeroseneConfig::default()
        };

        let (terminal, _) = TradingTerminal::boot_from_config(cfg);
        let instance = terminal.order_books.get(&7).expect("fixed book");

        assert_eq!(instance.mode, OrderBookSymbolMode::Fixed("@0".to_string()));
        assert_eq!(
            instance.pending_book_request_id(),
            None,
            "raw @0 must not be sent before spotMeta supplies PURR/USDC"
        );
    }
}
