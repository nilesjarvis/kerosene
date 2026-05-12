use crate::api::{OrderBook, fetch_order_book};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookId, OrderBookSymbolMode};
use crate::message::Message;
use iced::Task;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::market_update::order_book) struct OrderBookFetchPlan {
    pub(in crate::market_update::order_book) id: OrderBookId,
    pub(in crate::market_update::order_book) symbol: String,
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
        sigfigs,
    })
}

fn positive_finite(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

impl TradingTerminal {
    pub(in crate::market_update::order_book) fn apply_order_book_loaded(
        &mut self,
        id: OrderBookId,
        result: Result<OrderBook, String>,
    ) -> Task<Message> {
        if self.order_book_instance_is_muted(id) {
            return Task::none();
        }
        if let Some(inst) = self.order_books.get_mut(&id) {
            inst.book_loading = false;
            match result {
                Ok(book) => {
                    inst.set_book(book);
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
            let symbol = match &inst.mode {
                OrderBookSymbolMode::Active => self.active_symbol.clone(),
                OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
            };
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
            }
            return Task::none();
        };

        if let Some(inst) = self.order_books.get_mut(&id) {
            inst.book_loading = true;
            inst.book_error = None;
        }

        Task::perform(fetch_order_book(plan.symbol, plan.sigfigs), move |result| {
            Message::BookLoaded(plan.id, result)
        })
    }

    pub(crate) fn order_book_fetch_tasks_for_all(&mut self) -> Task<Message> {
        let ids: Vec<OrderBookId> = self.order_books.keys().copied().collect();
        Task::batch(
            ids.into_iter()
                .map(|id| self.order_book_fetch_task_for_id(id)),
        )
    }

    pub(crate) fn order_book_unavailable_reason(&self, symbol: &str) -> Option<String> {
        if symbol.is_empty() {
            return Some("No order-book symbol selected".to_string());
        }
        if self.is_ticker_muted(symbol) {
            return Some("Order book ticker is muted in Settings > Risk".to_string());
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
            self.is_ticker_muted(&symbol)
        })
    }
}
