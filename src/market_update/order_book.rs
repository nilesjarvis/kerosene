use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use iced::Task;

mod book_data;
mod panes;
mod ws_updates;

use ws_updates::order_book_tracks_coin;

impl TradingTerminal {
    pub(crate) fn update_order_book_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddOrderBookPane => self.add_order_book_pane(),
            Message::BookLoaded {
                request_id,
                id,
                coin,
                tick_size,
                sigfigs,
                result,
            } => self.apply_order_book_loaded(
                request_id,
                id,
                coin,
                tick_size,
                sigfigs,
                result.into_result(),
            ),
            Message::OrderBookWsAssetCtxUpdate {
                id,
                coin,
                source_context,
                ctx,
            } => {
                if !self.market_stream_source_is_current(source_context) {
                    return Task::none();
                }
                if self.symbol_key_is_hidden(&coin) {
                    return Task::none();
                }
                if self.order_book_instance_is_muted(id) {
                    return Task::none();
                }
                if let Some(inst) = self.order_books.get_mut(&id)
                    && order_book_tracks_coin(&inst.mode, &self.active_symbol, &coin)
                {
                    let now = std::time::Instant::now();
                    inst.asset_ctx = Some(ctx.clone());
                    inst.asset_ctx_updated_at = Some(now);
                    inst.record_spread_sample(now);
                    inst.record_mid_price_sample(now);
                }
                Task::none()
            }
            Message::OrderBookWsAssetCtxLagged {
                id,
                coin,
                source_context,
                skipped: _skipped,
            } => {
                if !self.market_stream_source_is_current(source_context) {
                    return Task::none();
                }
                if self.symbol_key_is_hidden(&coin) {
                    return Task::none();
                }
                if self.order_book_instance_is_muted(id) {
                    return Task::none();
                }
                if let Some(inst) = self.order_books.get_mut(&id)
                    && order_book_tracks_coin(&inst.mode, &self.active_symbol, &coin)
                {
                    inst.clear_asset_context();
                }
                Task::none()
            }
            Message::WsBookUpdate {
                id,
                coin,
                sigfigs,
                source_context,
                book,
            } => {
                if !self.market_stream_source_is_current(source_context) {
                    return Task::none();
                }
                if self.symbol_key_is_hidden(&coin) {
                    return Task::none();
                }
                if sigfigs != self.canonical_l2_book_sigfigs(&coin) {
                    return Task::none();
                }
                let source_tick = helpers::sigfig_server_tick(sigfigs, book.mid_price());
                let now_ms = Self::now_ms();
                let mut newly_populated = false;
                let mut tick_mid = None;
                if let Some(inst) = self.order_books.get_mut(&id)
                    && order_book_tracks_coin(&inst.mode, &self.active_symbol, &coin)
                {
                    let was_empty = inst.book.bids.is_empty() && inst.book.asks.is_empty();
                    let now = std::time::Instant::now();
                    inst.apply_book_update_preserving_scope(book, source_tick);
                    inst.record_mid_price_sample(now);
                    inst.record_spread_sample(now);
                    tick_mid = inst.current_mid_price();
                    inst.book_loading = false;
                    inst.book_error = None;
                    newly_populated =
                        was_empty && !(inst.book.bids.is_empty() && inst.book.asks.is_empty());
                }
                if let Some(mid) = tick_mid {
                    self.apply_orderbook_tick_price_to_charts(&coin, mid, now_ms);
                }

                // A book that first fills in over the websocket (REST blip,
                // unmute, symbol scrub) should open centered on the spread,
                // exactly like the REST load path.
                if newly_populated {
                    return self.center_order_book(id);
                }
                Task::none()
            }
            Message::OrderBookWsBookLagged {
                id,
                coin,
                sigfigs,
                source_context,
                skipped,
            } => {
                if !self.market_stream_source_is_current(source_context) {
                    return Task::none();
                }
                if self.symbol_key_is_hidden(&coin) {
                    return Task::none();
                }
                if sigfigs != self.canonical_l2_book_sigfigs(&coin) {
                    return Task::none();
                }
                if let Some(inst) = self.order_books.get_mut(&id)
                    && order_book_tracks_coin(&inst.mode, &self.active_symbol, &coin)
                {
                    inst.book_loading = true;
                    inst.book_error = Some(format!(
                        "Order book stream lagged; reconnecting after skipping {skipped} L2 updates"
                    ));
                }
                Task::none()
            }
            Message::SetBookTickSize(id, tick) => {
                if !helpers::valid_book_tick_size(tick) {
                    self.order_status = Some(("Invalid order-book tick size".into(), true));
                    return Task::none();
                }
                let symbol = self
                    .order_books
                    .get(&id)
                    .map(|inst| match &inst.mode {
                        OrderBookSymbolMode::Active => self.active_symbol.clone(),
                        OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
                    })
                    .unwrap_or_default();
                if self.symbol_key_is_hidden(&symbol) {
                    if let Some(inst) = self.order_books.get_mut(&id) {
                        inst.book_loading = false;
                    }
                    return Task::none();
                }
                if let Some(inst) = self.order_books.get_mut(&id) {
                    if helpers::tick_sizes_match(inst.tick_size, tick) {
                        return Task::none();
                    }
                    let old_tick = inst.tick_size;
                    let denomination_increased = tick > old_tick;
                    let should_fetch = inst.book.bids.is_empty()
                        || inst.book.asks.is_empty()
                        || denomination_increased
                        || !inst.can_render_book_at_tick(tick)
                        || inst.book_error.is_some();
                    inst.set_tick_size(tick);
                    inst.clear_book_request();
                    inst.book_loading = should_fetch;
                    if should_fetch {
                        inst.book_error = None;
                    }

                    self.persist_config();
                    if should_fetch {
                        return Task::batch([
                            self.center_order_book(id),
                            self.order_book_fetch_task_for_id(id),
                        ]);
                    }
                    return self.center_order_book(id);
                }
                Task::none()
            }
            Message::ToggleOrderBookSettings(id) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.settings_open = !inst.settings_open;
                }
                Task::none()
            }
            Message::ToggleOrderBookCenterOnMid(id) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.center_on_mid = !inst.center_on_mid;
                    let center_on_mid = inst.center_on_mid;

                    self.persist_config();
                    if !center_on_mid {
                        return self.center_order_book(id);
                    }
                }
                Task::none()
            }
            Message::ToggleOrderBookReverseSide(id) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.reverse_side = !inst.reverse_side;
                }
                self.persist_config();
                Task::none()
            }
            Message::ToggleOrderBookSpreadChart(id) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.show_spread_chart = !inst.show_spread_chart;
                }
                self.persist_config();
                Task::none()
            }
            Message::OrderBookSpreadChartResize(id, new_height) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.set_spread_chart_height(new_height);
                }
                self.persist_config();
                Task::none()
            }
            Message::OrderBookSearchChanged(id, query) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.search_query = query;
                }
                Task::none()
            }
            Message::OrderBookSetMode(id, mode) => {
                let symbol = match &mode {
                    OrderBookSymbolMode::Active => self.active_symbol.clone(),
                    OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
                };
                if self.symbol_key_is_hidden(&symbol) {
                    self.order_status = Some((
                        "Order book ticker is hidden in Settings > Risk".into(),
                        true,
                    ));
                    return Task::none();
                }
                if self
                    .exchange_symbols
                    .iter()
                    .find(|exchange_symbol| exchange_symbol.key == symbol)
                    .is_some_and(|exchange_symbol| !exchange_symbol.is_user_selectable_market())
                {
                    self.order_status =
                        Some(("Order book market is not tradable".to_string(), true));
                    return Task::none();
                }
                let mid = self.resolve_mid_for_symbol(&symbol);
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.mode = mode.clone();
                    inst.settings_open = false;
                    inst.set_book(OrderBook::empty());
                    inst.clear_asset_context_and_price_history();
                    inst.reset_tick_options_basis();
                    // Drop any in-flight request marker so the fetch dedup
                    // guard cannot mistake the old symbol's request for ours.
                    inst.clear_book_request();
                    inst.book_loading = true;
                    inst.book_error = None;
                    inst.book_failure_toasted = false;

                    inst.set_tick_size(mid.map(helpers::default_tick_for_price).unwrap_or(0.01));

                    self.persist_config();
                    return self.order_book_fetch_task_for_id(id);
                }
                Task::none()
            }
            Message::SetOrderBookDisplayMode(id, display_mode) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    if inst.display_mode == display_mode {
                        return Task::none();
                    }
                    inst.display_mode = display_mode;
                    self.persist_config();
                    return self.center_order_book(id);
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AssetContext;
    use crate::api::BookLevel;
    use crate::chart::ChartStatus;
    use crate::chart_state::ChartInstance;
    use crate::config::ReadDataProvider;
    use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
    use crate::timeframe::Timeframe;

    fn book() -> OrderBook {
        OrderBook {
            bids: vec![BookLevel { px: 99.0, sz: 1.0 }],
            asks: vec![BookLevel { px: 101.0, sz: 1.0 }],
        }
    }

    fn asset_ctx(mid_px: &str) -> AssetContext {
        AssetContext {
            funding: None,
            open_interest: None,
            oracle_px: None,
            mark_px: None,
            mid_px: Some(mid_px.to_string()),
            prev_day_px: None,
            day_ntl_vlm: None,
            day_base_vlm: None,
            impact_pxs: None,
        }
    }

    fn asset_ctx_with_impact(bid: &str, ask: &str) -> AssetContext {
        AssetContext {
            funding: None,
            open_interest: None,
            oracle_px: None,
            mark_px: None,
            mid_px: None,
            prev_day_px: None,
            day_ntl_vlm: None,
            day_base_vlm: None,
            impact_pxs: Some(vec![bid.to_string(), ask.to_string()]),
        }
    }

    fn terminal_with_order_book() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.order_books.clear();
        terminal.active_symbol = "BTC".to_string();
        terminal.order_books.insert(
            7,
            OrderBookInstance::new(7, OrderBookSymbolMode::Active, 1.0),
        );
        terminal
    }

    fn source_context(
        terminal: &TradingTerminal,
        hydromancer_key_generation: Option<u64>,
    ) -> crate::read_data_provider::MarketDataSourceContext {
        crate::read_data_provider::MarketDataSourceContext {
            hydromancer_key_generation,
            ..terminal.market_data_source_context()
        }
    }

    #[test]
    fn stale_hydromancer_generation_does_not_update_order_book_snapshot() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::WsBookUpdate {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: terminal.canonical_l2_book_sigfigs("BTC"),
            source_context: source_context(&terminal, Some(1)),
            book: book(),
        });

        assert!(terminal.order_books[&7].book.bids.is_empty());
        assert!(terminal.order_books[&7].book.asks.is_empty());
    }

    #[test]
    fn stale_hyperliquid_generation_does_not_update_order_book_snapshot() {
        let mut terminal = terminal_with_order_book();
        let stale_context = source_context(&terminal, None);
        terminal.bump_read_data_provider_generation();

        let _task = terminal.update_order_book_market(Message::WsBookUpdate {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: terminal.canonical_l2_book_sigfigs("BTC"),
            source_context: stale_context,
            book: book(),
        });

        assert!(terminal.order_books[&7].book.bids.is_empty());
        assert!(terminal.order_books[&7].book.asks.is_empty());
    }

    #[test]
    fn current_hydromancer_generation_updates_order_book_snapshot() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::WsBookUpdate {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: terminal.canonical_l2_book_sigfigs("BTC"),
            source_context: source_context(&terminal, Some(2)),
            book: book(),
        });

        assert_eq!(terminal.order_books[&7].book.bids.len(), 1);
        assert_eq!(terminal.order_books[&7].book.asks.len(), 1);
    }

    #[test]
    fn order_book_snapshot_updates_matching_tick_chart() {
        let mut terminal = terminal_with_order_book();
        terminal.charts.clear();
        let mut chart = ChartInstance::new(1, "BTC".to_string(), Timeframe::Tick);
        chart.chart.status = ChartStatus::Loaded;
        terminal.charts.insert(1, chart);

        let _task = terminal.update_order_book_market(Message::WsBookUpdate {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: terminal.canonical_l2_book_sigfigs("BTC"),
            source_context: source_context(&terminal, None),
            book: book(),
        });

        let candles = &terminal.charts[&1].chart.candles;
        assert_eq!(candles.len(), 1);
        assert_eq!(candles[0].close, 100.0);
    }

    #[test]
    fn current_hydromancer_lag_marks_order_book_stale_without_dropping_snapshot() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;
        let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");

        let _task = terminal.update_order_book_market(Message::WsBookUpdate {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs,
            source_context: source_context(&terminal, Some(2)),
            book: book(),
        });
        let current_sigfigs = terminal.canonical_l2_book_sigfigs("BTC");
        let _task = terminal.update_order_book_market(Message::OrderBookWsBookLagged {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: current_sigfigs,
            source_context: source_context(&terminal, Some(2)),
            skipped: 17,
        });

        let inst = &terminal.order_books[&7];
        assert_eq!(inst.book.bids.len(), 1);
        assert_eq!(inst.book.asks.len(), 1);
        assert!(inst.book_loading);
        assert_eq!(
            inst.book_error.as_deref(),
            Some("Order book stream lagged; reconnecting after skipping 17 L2 updates")
        );
    }

    #[test]
    fn root_update_dispatches_order_book_lag_to_market_handler() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;
        let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");

        let _task = terminal.update(Message::OrderBookWsBookLagged {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs,
            source_context: source_context(&terminal, Some(2)),
            skipped: 17,
        });

        let inst = &terminal.order_books[&7];
        assert!(inst.book_loading);
        assert_eq!(
            inst.book_error.as_deref(),
            Some("Order book stream lagged; reconnecting after skipping 17 L2 updates")
        );
    }

    #[test]
    fn stale_hydromancer_lag_does_not_mark_order_book_stale() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;
        let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");

        let _task = terminal.update_order_book_market(Message::OrderBookWsBookLagged {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs,
            source_context: source_context(&terminal, Some(1)),
            skipped: 17,
        });

        let inst = &terminal.order_books[&7];
        assert!(!inst.book_loading);
        assert!(inst.book_error.is_none());
    }

    #[test]
    fn stale_hydromancer_generation_does_not_update_order_book_asset_context() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::OrderBookWsAssetCtxUpdate {
            id: 7,
            coin: "BTC".to_string(),
            source_context: source_context(&terminal, Some(1)),
            ctx: asset_ctx("100"),
        });

        assert!(terminal.order_books[&7].asset_ctx.is_none());
    }

    #[test]
    fn current_hydromancer_generation_updates_order_book_asset_context() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::OrderBookWsAssetCtxUpdate {
            id: 7,
            coin: "BTC".to_string(),
            source_context: source_context(&terminal, Some(2)),
            ctx: asset_ctx("100"),
        });

        assert_eq!(
            terminal.order_books[&7]
                .asset_ctx
                .as_ref()
                .and_then(|ctx| ctx.mid_px.as_deref()),
            Some("100")
        );
    }

    #[test]
    fn order_book_asset_context_update_for_untracked_coin_is_ignored() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::OrderBookWsAssetCtxUpdate {
            id: 7,
            coin: "ETH".to_string(),
            source_context: source_context(&terminal, Some(2)),
            ctx: asset_ctx("100"),
        });

        assert!(terminal.order_books[&7].asset_ctx.is_none());
    }

    #[test]
    fn current_asset_context_lag_clears_order_book_impact_context_only() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::WsBookUpdate {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: terminal.canonical_l2_book_sigfigs("BTC"),
            source_context: source_context(&terminal, Some(2)),
            book: book(),
        });
        let _task = terminal.update_order_book_market(Message::OrderBookWsAssetCtxUpdate {
            id: 7,
            coin: "BTC".to_string(),
            source_context: source_context(&terminal, Some(2)),
            ctx: asset_ctx_with_impact("90", "110"),
        });
        assert_eq!(
            terminal.order_books[&7].best_bid_ask(),
            (Some(90.0), Some(110.0))
        );
        assert!(!terminal.order_books[&7].spread_history.is_empty());

        let _task = terminal.update(Message::OrderBookWsAssetCtxLagged {
            id: 7,
            coin: "BTC".to_string(),
            source_context: source_context(&terminal, Some(2)),
            skipped: 5,
        });

        let inst = &terminal.order_books[&7];
        assert!(inst.asset_ctx.is_none());
        assert!(!inst.spread_history.is_empty());
        assert_eq!(inst.best_bid_ask(), (Some(99.0), Some(101.0)));
        assert_eq!(inst.book.bids.len(), 1);
        assert_eq!(inst.book.asks.len(), 1);
    }

    #[test]
    fn stale_asset_context_lag_does_not_clear_order_book_context() {
        let mut terminal = terminal_with_order_book();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::OrderBookWsAssetCtxUpdate {
            id: 7,
            coin: "BTC".to_string(),
            source_context: source_context(&terminal, Some(2)),
            ctx: asset_ctx_with_impact("90", "110"),
        });
        let _task = terminal.update(Message::OrderBookWsAssetCtxLagged {
            id: 7,
            coin: "BTC".to_string(),
            source_context: source_context(&terminal, Some(1)),
            skipped: 5,
        });

        assert!(terminal.order_books[&7].asset_ctx.is_some());
        assert!(!terminal.order_books[&7].spread_history.is_empty());
    }

    #[test]
    fn order_book_snapshot_gates_provider_source() {
        let mut terminal = terminal_with_order_book();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::WsBookUpdate {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: terminal.canonical_l2_book_sigfigs("BTC"),
            source_context: source_context(&terminal, Some(2)),
            book: book(),
        });

        assert!(terminal.order_books[&7].book.bids.is_empty());
        assert!(terminal.order_books[&7].book.asks.is_empty());

        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        let _task = terminal.update_order_book_market(Message::WsBookUpdate {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: terminal.canonical_l2_book_sigfigs("BTC"),
            source_context: source_context(&terminal, None),
            book: book(),
        });

        assert_eq!(terminal.order_books[&7].book.bids.len(), 1);
        assert_eq!(terminal.order_books[&7].book.asks.len(), 1);
    }

    #[test]
    fn order_book_asset_context_gates_provider_source() {
        let mut terminal = terminal_with_order_book();
        terminal.hydromancer_key_generation = 2;

        let _task = terminal.update_order_book_market(Message::OrderBookWsAssetCtxUpdate {
            id: 7,
            coin: "BTC".to_string(),
            source_context: source_context(&terminal, Some(2)),
            ctx: asset_ctx("100"),
        });

        assert!(terminal.order_books[&7].asset_ctx.is_none());

        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "hydro-key".to_string().into();
        let _task = terminal.update_order_book_market(Message::OrderBookWsAssetCtxUpdate {
            id: 7,
            coin: "BTC".to_string(),
            source_context: source_context(&terminal, None),
            ctx: asset_ctx("101"),
        });

        assert_eq!(
            terminal.order_books[&7]
                .asset_ctx
                .as_ref()
                .and_then(|ctx| ctx.mid_px.as_deref()),
            Some("101")
        );
    }
}
