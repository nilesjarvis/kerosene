use crate::api::{OrderBook, fetch_order_book};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookId, OrderBookSymbolMode};
use crate::message::Message;
use iced::Task;
use planning::{
    order_book_needs_precision_refresh, order_book_response_matches_expected_precision,
    plan_order_book_fetch,
};

mod availability;
mod planning;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn canonical_l2_book_sigfigs(&self, symbol: &str) -> (Option<u8>, Option<u8>) {
        let Some(mid) = self
            .order_books
            .values()
            .filter(|book| self.order_book_symbol_for_mode(&book.mode) == symbol)
            .filter_map(|book| helpers::positive_finite_value(book.book.mid_price()))
            .next()
            .or_else(|| {
                self.resolve_mid_for_symbol(symbol)
                    .and_then(helpers::positive_finite_value)
            })
        else {
            return (None, None);
        };

        helpers::compute_sigfigs(helpers::default_tick_for_price(mid), mid)
    }

    /// Reset every Active-mode order book for a switch to `symbol`: empty the
    /// book, drop per-symbol state, and re-seed the tick at the new symbol's
    /// default so the first fetch is planned at the right precision. Clears
    /// any in-flight request marker too — a stale pending request from the
    /// previous symbol would otherwise satisfy the fetch dedup guard and
    /// silently skip the new symbol's fetch. Every active-symbol-switch path
    /// must go through this.
    pub(crate) fn reset_active_order_books_for_symbol(&mut self, symbol: &str) {
        let default_tick = self
            .resolve_mid_for_symbol(symbol)
            .map(helpers::default_tick_for_price)
            .unwrap_or(0.01);
        for inst in self.order_books.values_mut() {
            if inst.mode == OrderBookSymbolMode::Active {
                inst.set_book(OrderBook::empty());
                inst.clear_asset_context_and_price_history();
                inst.reset_tick_options_basis();
                inst.set_tick_size(default_tick);
                inst.clear_book_request();
                inst.book_loading = true;
                inst.book_error = None;
                inst.book_failure_toasted = false;
            }
        }
    }

    pub(in crate::market_update::order_book) fn apply_order_book_loaded(
        &mut self,
        request_id: u64,
        id: OrderBookId,
        coin: String,
        tick_size: f64,
        sigfigs: (Option<u8>, Option<u8>),
        result: Result<OrderBook, String>,
    ) -> Task<Message> {
        if self.order_book_instance_is_muted(id) {
            return Task::none();
        }
        let tracks_coin = self.order_books.get(&id).is_some_and(|inst| {
            super::ws_updates::order_book_tracks_coin(&inst.mode, &self.active_symbol, &coin)
        });
        if !tracks_coin {
            return Task::none();
        }
        let tick_still_current = self
            .order_books
            .get(&id)
            .is_some_and(|inst| helpers::tick_sizes_match(inst.tick_size, tick_size));
        if !tick_still_current {
            return Task::none();
        }
        let request_still_current = self.order_books.get(&id).is_some_and(|inst| {
            inst.pending_book_request_matches_id(request_id, &coin, tick_size, sigfigs)
        });
        if !request_still_current {
            return Task::none();
        }
        if !order_book_response_matches_expected_precision(
            tick_size,
            sigfigs,
            self.resolve_mid_for_symbol(&coin),
        ) {
            if let Some(inst) = self.order_books.get_mut(&id) {
                inst.clear_matching_book_request(request_id, &coin, tick_size, sigfigs);
            }
            return self.order_book_fetch_task_for_id(id);
        }
        if let Some(inst) = self.order_books.get_mut(&id) {
            inst.clear_matching_book_request(request_id, &coin, tick_size, sigfigs);
            inst.book_loading = false;
            match result {
                Ok(book) => {
                    let was_empty = inst.book.bids.is_empty() && inst.book.asks.is_empty();
                    let source_tick = helpers::sigfig_server_tick(sigfigs, book.mid_price());
                    inst.set_book_with_source(book, source_tick);
                    let now = std::time::Instant::now();
                    inst.record_mid_price_sample(now);
                    inst.record_spread_sample(now);
                    inst.book_error = None;
                    inst.book_failure_toasted = false;

                    let tick_options = helpers::book_tick_options(inst.tick_options_mid());
                    let is_valid_tick = tick_options
                        .iter()
                        .any(|&opt| (opt - inst.tick_size).abs() / opt.max(1e-12) < 0.01);

                    if !is_valid_tick {
                        // Stale persisted tick from a different price regime:
                        // snap to the nearest option so the selector keeps an
                        // active button and the coarseness choice survives.
                        inst.set_tick_size(helpers::nearest_tick_option(
                            &tick_options,
                            inst.tick_size,
                        ));
                    }

                    // Only snap the scroll position when the book goes from
                    // empty to populated; background precision and scope
                    // refreshes land silently into the user's position.
                    if was_empty {
                        return self.center_order_book(id);
                    }
                }
                Err(error) => {
                    let message = format!(
                        "Order book load failed: {}",
                        helpers::redact_sensitive_response_text(&error)
                    );
                    let should_toast = !inst.book_failure_toasted;
                    inst.book_failure_toasted = true;
                    inst.book_error = Some(message.clone());
                    if should_toast {
                        self.push_toast(message, true);
                    }
                }
            }
        }
        Task::none()
    }

    pub(in crate::market_update::order_book) fn center_order_book(
        &self,
        id: OrderBookId,
    ) -> Task<Message> {
        if let Some(inst) = self.order_books.get(&id) {
            return iced::widget::operation::snap_to(
                inst.scroll_id.clone(),
                iced::widget::scrollable::RelativeOffset { x: 0.0, y: 0.5 },
            );
        }
        Task::none()
    }

    pub(crate) fn order_book_fetch_task_for_id(&mut self, id: OrderBookId) -> Task<Message> {
        let Some((mode, tick_size, book_mid, symbol)) = self.order_books.get(&id).map(|inst| {
            let symbol = self.order_book_symbol_for_mode(&inst.mode);
            (
                inst.mode.clone(),
                inst.tick_size,
                inst.book.mid_price(),
                symbol,
            )
        }) else {
            return Task::none();
        };

        if let Some(reason) = self.order_book_unavailable_reason(&symbol) {
            if let Some(inst) = self.order_books.get_mut(&id) {
                inst.book_loading = false;
                inst.clear_book_request();
                inst.book_error = Some(reason);
            }
            return Task::none();
        }

        let Some(plan) = plan_order_book_fetch(
            id,
            &mode,
            &self.active_symbol,
            tick_size,
            book_mid,
            self.resolve_mid_for_symbol(&symbol),
            false,
        ) else {
            if let Some(inst) = self.order_books.get_mut(&id) {
                inst.book_loading = false;
                inst.clear_book_request();
            }
            return Task::none();
        };

        if self.order_books.get(&id).is_some_and(|inst| {
            inst.book_loading
                && inst.pending_book_request_matches(&plan.symbol, plan.tick_size, plan.sigfigs)
        }) {
            return Task::none();
        }

        if let Some(inst) = self.order_books.get_mut(&id) {
            inst.book_loading = true;
            inst.book_error = None;
            let request_id =
                inst.mark_book_request(plan.symbol.clone(), plan.tick_size, plan.sigfigs);
            return Task::perform(
                fetch_order_book(plan.symbol.clone(), plan.sigfigs),
                move |result| Message::BookLoaded {
                    request_id,
                    id: plan.id,
                    coin: plan.symbol,
                    tick_size: plan.tick_size,
                    sigfigs: plan.sigfigs,
                    result,
                },
            );
        }

        Task::none()
    }

    pub(crate) fn order_book_fetch_tasks_for_all(&mut self) -> Task<Message> {
        let ids: Vec<OrderBookId> = self.order_books.keys().copied().collect();
        Task::batch(
            ids.into_iter()
                .map(|id| self.order_book_fetch_task_for_id(id)),
        )
    }

    pub(crate) fn order_book_precision_refresh_task(&mut self) -> Task<Message> {
        let ids = self.order_book_precision_refresh_ids();
        Task::batch(
            ids.into_iter()
                .map(|id| self.order_book_fetch_task_for_id(id)),
        )
    }

    pub(crate) fn order_book_precision_refresh_ids(&self) -> Vec<OrderBookId> {
        self.order_books
            .iter()
            .filter_map(|(&id, inst)| {
                let symbol = self.order_book_symbol_for_mode(&inst.mode);
                if inst.book_error.is_some()
                    || self.order_book_unavailable_reason(&symbol).is_some()
                {
                    return None;
                }
                order_book_needs_precision_refresh(
                    inst.tick_size,
                    inst.book_source_tick_size(),
                    inst.book_source_mid(),
                    inst.pending_book_sigfigs(),
                    inst.book_loading,
                    self.resolve_mid_for_symbol(&symbol),
                )
                .then_some(id)
            })
            .collect()
    }
}
