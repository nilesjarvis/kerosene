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
use ws_updates::{best_chase_price, chase_should_reprice, order_book_tracks_coin};

impl TradingTerminal {
    pub(crate) fn update_order_book_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddOrderBookPane => self.add_order_book_pane(),
            Message::BookLoaded(id, result) => self.apply_order_book_loaded(id, result),
            Message::OrderBookWsAssetCtxUpdate(id, ctx) => {
                if self.order_book_instance_is_muted(id) {
                    return Task::none();
                }
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.asset_ctx = Some(ctx.clone());
                    record_asset_context_spread(
                        &mut inst.spread_history,
                        &ctx,
                        std::time::Instant::now(),
                    );
                }
                Task::none()
            }
            Message::WsBookUpdate(id, coin, book) => {
                if self.is_ticker_muted(&coin) {
                    return Task::none();
                }
                if let Some(inst) = self.order_books.get_mut(&id)
                    && order_book_tracks_coin(&inst.mode, &self.active_symbol, &coin)
                {
                    inst.book = book.clone();
                    inst.book_loading = false;
                    inst.book_error = None;
                }

                let Some(chase) = &self.active_chase else {
                    return Task::none();
                };
                let best = self
                    .order_books
                    .values()
                    .find(|book| book.mode == OrderBookSymbolMode::Active)
                    .and_then(|active_book| best_chase_price(&active_book.book, chase.is_buy));

                if chase_should_reprice(chase, &self.active_symbol, &coin, best) {
                    return self.chase_cancel_and_reprice();
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
                if self.is_ticker_muted(&symbol) {
                    if let Some(inst) = self.order_books.get_mut(&id) {
                        inst.book_loading = false;
                    }
                    return Task::none();
                }
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.tick_size = tick;
                    inst.book_loading = true;
                    inst.book_error = None;

                    self.persist_config();
                    return self.order_book_fetch_task_for_id(id);
                }
                Task::none()
            }
            Message::ToggleOrderBookSettings(id) => {
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.settings_open = !inst.settings_open;
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
                if self.is_ticker_muted(&symbol) {
                    self.order_status =
                        Some(("Order book ticker is muted in Settings > Risk".into(), true));
                    return Task::none();
                }
                let mid = self.resolve_mid_for_symbol(&symbol);
                if let Some(inst) = self.order_books.get_mut(&id) {
                    inst.mode = mode.clone();
                    inst.settings_open = false;
                    inst.book = OrderBook::empty();
                    inst.asset_ctx = None;
                    inst.spread_history.clear();
                    inst.book_loading = true;
                    inst.book_error = None;

                    inst.tick_size = mid.map(helpers::default_tick_for_price).unwrap_or(0.01);

                    self.persist_config();
                    return self.order_book_fetch_task_for_id(id);
                }
                Task::none()
            }
            Message::CenterOrderBook(id) => self.center_order_book(id),
            _ => Task::none(),
        }
    }
}
