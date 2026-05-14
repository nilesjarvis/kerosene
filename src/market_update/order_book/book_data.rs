use crate::api::{OrderBook, fetch_order_book};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookId, OrderBookSymbolMode};
use crate::message::Message;
use iced::Task;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::market_update::order_book) struct OrderBookFetchPlan {
    pub(in crate::market_update::order_book) id: OrderBookId,
    pub(in crate::market_update::order_book) symbol: String,
    pub(in crate::market_update::order_book) tick_size: f64,
    pub(in crate::market_update::order_book) sigfigs: (Option<u8>, Option<u8>),
}

pub(in crate::market_update::order_book) fn plan_order_book_fetch(
    id: OrderBookId,
    mode: &OrderBookSymbolMode,
    active_symbol: &str,
    tick_size: f64,
    book_mid: f64,
    fallback_mid: Option<f64>,
    unavailable: bool,
) -> Option<OrderBookFetchPlan> {
    let symbol = match mode {
        OrderBookSymbolMode::Active => active_symbol.to_string(),
        OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
    };
    if symbol.is_empty() || unavailable {
        return None;
    }

    let mid = positive_finite(book_mid).or_else(|| fallback_mid.and_then(positive_finite));
    let sigfigs = mid
        .map(|mid| helpers::compute_sigfigs(tick_size, mid))
        .unwrap_or((None, None));

    Some(OrderBookFetchPlan {
        id,
        symbol,
        tick_size,
        sigfigs,
    })
}

fn positive_finite(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

pub(in crate::market_update::order_book) fn order_book_needs_precision_refresh(
    selected_tick: f64,
    source_tick: Option<f64>,
    pending_sigfigs: Option<(Option<u8>, Option<u8>)>,
    book_loading: bool,
    mid: Option<f64>,
) -> bool {
    if book_loading {
        return false;
    }

    let Some(mid) = mid.and_then(positive_finite) else {
        return false;
    };
    if !saved_tick_requires_aggregated_fetch(selected_tick, mid) {
        return false;
    }

    let expected_sigfigs = helpers::compute_sigfigs(selected_tick, mid);
    if pending_sigfigs == Some(expected_sigfigs) {
        return false;
    }

    let Some(expected_source_tick) = helpers::sigfig_server_tick(expected_sigfigs, mid) else {
        return false;
    };
    !source_tick.is_some_and(|actual| helpers::tick_sizes_match(actual, expected_source_tick))
}

fn saved_tick_requires_aggregated_fetch(selected_tick: f64, mid: f64) -> bool {
    if !helpers::valid_book_tick_size(selected_tick) {
        return false;
    }
    let default_tick = helpers::default_tick_for_price(mid);
    selected_tick > default_tick && !helpers::tick_sizes_match(selected_tick, default_tick)
}

fn order_book_response_matches_expected_precision(
    tick_size: f64,
    sigfigs: (Option<u8>, Option<u8>),
    mid: Option<f64>,
) -> bool {
    let Some(mid) = mid.and_then(positive_finite) else {
        return true;
    };
    if !saved_tick_requires_aggregated_fetch(tick_size, mid) {
        return true;
    }

    sigfigs == helpers::compute_sigfigs(tick_size, mid)
}

impl TradingTerminal {
    pub(crate) fn order_book_symbol_for_mode(&self, mode: &OrderBookSymbolMode) -> String {
        match mode {
            OrderBookSymbolMode::Active => self.active_symbol.clone(),
            OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
        }
    }

    pub(crate) fn canonical_l2_book_sigfigs(&self, symbol: &str) -> (Option<u8>, Option<u8>) {
        let Some(mid) = self
            .order_books
            .values()
            .filter(|book| self.order_book_symbol_for_mode(&book.mode) == symbol)
            .filter_map(|book| positive_finite(book.book.mid_price()))
            .next()
            .or_else(|| {
                self.resolve_mid_for_symbol(symbol)
                    .and_then(positive_finite)
            })
        else {
            return (None, None);
        };

        helpers::compute_sigfigs(helpers::default_tick_for_price(mid), mid)
    }

    pub(in crate::market_update::order_book) fn apply_order_book_loaded(
        &mut self,
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
        if !order_book_response_matches_expected_precision(
            tick_size,
            sigfigs,
            self.resolve_mid_for_symbol(&coin),
        ) {
            if let Some(inst) = self.order_books.get_mut(&id) {
                inst.clear_matching_book_request(sigfigs);
            }
            return self.order_book_fetch_task_for_id(id);
        }
        if let Some(inst) = self.order_books.get_mut(&id) {
            inst.clear_matching_book_request(sigfigs);
            inst.book_loading = false;
            match result {
                Ok(book) => {
                    let source_tick = helpers::sigfig_server_tick(sigfigs, book.mid_price());
                    inst.set_book_with_source(book, source_tick);
                    inst.book_error = None;
                    let mid = inst.book.mid_price();

                    let tick_options = helpers::book_tick_options(mid);
                    let is_valid_tick = tick_options
                        .iter()
                        .any(|&opt| (opt - inst.tick_size).abs() / opt.max(1e-12) < 0.01);

                    if !is_valid_tick {
                        inst.set_tick_size(helpers::default_tick_for_price(mid));
                    }

                    return self.center_order_book(id);
                }
                Err(error) => {
                    let message = format!("Order book load failed: {error}");
                    inst.book_error = Some(message.clone());
                    self.push_toast(message, true);
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
            inst.book_loading && inst.pending_book_sigfigs() == Some(plan.sigfigs)
        }) {
            return Task::none();
        }

        if let Some(inst) = self.order_books.get_mut(&id) {
            inst.book_loading = true;
            inst.book_error = None;
            inst.mark_book_request(plan.sigfigs);
        }

        Task::perform(
            fetch_order_book(plan.symbol.clone(), plan.sigfigs),
            move |result| Message::BookLoaded {
                id: plan.id,
                coin: plan.symbol,
                tick_size: plan.tick_size,
                sigfigs: plan.sigfigs,
                result,
            },
        )
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
                    inst.pending_book_sigfigs(),
                    inst.book_loading,
                    self.resolve_mid_for_symbol(&symbol),
                )
                .then_some(id)
            })
            .collect()
    }

    pub(crate) fn order_book_unavailable_reason(&self, symbol: &str) -> Option<String> {
        if symbol.is_empty() {
            return Some("No order-book symbol selected".to_string());
        }
        if self.symbol_key_is_hidden(symbol) {
            return Some("Order book ticker is hidden in Settings > Risk".to_string());
        }
        if self.is_outcome_coin(symbol) {
            return Some("Order book depth is not available for outcome markets".to_string());
        }
        None
    }

    pub(in crate::market_update::order_book) fn order_book_instance_is_muted(
        &self,
        id: OrderBookId,
    ) -> bool {
        self.order_books.get(&id).is_some_and(|inst| {
            let symbol = match &inst.mode {
                OrderBookSymbolMode::Active => self.active_symbol.clone(),
                OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
            };
            self.symbol_key_is_hidden(&symbol)
        })
    }
}
