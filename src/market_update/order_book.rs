use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use iced::Task;

mod asset_ctx;
mod book_data;
mod panes;
mod ws_updates;

use asset_ctx::record_asset_context_spread;
use ws_updates::order_book_tracks_coin;

impl TradingTerminal {
    pub(crate) fn update_order_book_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddOrderBookPane => self.add_order_book_pane(),
            Message::BookLoaded {
                id,
                coin,
                tick_size,
                sigfigs,
                result,
            } => self.apply_order_book_loaded(id, coin, tick_size, sigfigs, result),
            Message::OrderBookWsAssetCtxUpdate(id, ctx) => {
                if self.order_book_instance_is_muted(id) {
                    return Task::none();
                }
                if let Some(inst) = self.order_books.get_mut(&id) {
                    let now = std::time::Instant::now();
                    inst.asset_ctx = Some(ctx.clone());
                    record_asset_context_spread(&mut inst.spread_history, &ctx, now);
                    inst.record_mid_price_sample(now);
                }
                Task::none()
            }
            Message::WsBookUpdate {
                id,
                coin,
                sigfigs,
                book,
            } => {
                if self.symbol_key_is_hidden(&coin) {
                    return Task::none();
                }
                if sigfigs != self.canonical_l2_book_sigfigs(&coin) {
                    return Task::none();
                }
                let source_tick = helpers::sigfig_server_tick(sigfigs, book.mid_price());
                if let Some(inst) = self.order_books.get_mut(&id)
                    && order_book_tracks_coin(&inst.mode, &self.active_symbol, &coin)
                {
                    let now = std::time::Instant::now();
                    inst.apply_book_update_preserving_scope(book.clone(), source_tick);
                    inst.record_mid_price_sample(now);
                    inst.book_loading = false;
                    inst.book_error = None;
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
            Message::ToggleOrderBookSpreadChart(id) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.show_spread_chart = !inst.show_spread_chart;
                }
                self.persist_config();
                Task::none()
            }
            Message::OrderBookSpreadChartResize(id, new_height) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.spread_chart_height = new_height;
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
                let mid = self.resolve_mid_for_symbol(&symbol);
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.mode = mode.clone();
                    inst.settings_open = false;
                    inst.set_book(OrderBook::empty());
                    inst.asset_ctx = None;
                    inst.spread_history.clear();
                    inst.clear_mid_price_history();
                    inst.book_loading = true;
                    inst.book_error = None;

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
