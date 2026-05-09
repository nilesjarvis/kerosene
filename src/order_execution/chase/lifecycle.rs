use crate::app_state::TradingTerminal;
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
use crate::message::Message;
use crate::signing::{
    ChaseOrder, MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES, OrderKind,
    cancel_order, float_to_wire, place_order, round_price,
};
use iced::Task;
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StopChaseAction {
    CancelResting { asset: u32, oid: u64 },
    AwaitPlaceResult,
    AwaitCancelResult,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ChaseLimitReason {
    InvalidPrice,
    Timeout { elapsed: Duration },
    MaxReprices { count: u32 },
    Drift { drift_fraction: f64 },
}

impl ChaseLimitReason {
    fn status_detail(self) -> String {
        match self {
            Self::InvalidPrice => "invalid chase price".to_string(),
            Self::Timeout { elapsed } => {
                format!("timeout after {}s", elapsed.as_secs())
            }
            Self::MaxReprices { count } => {
                format!("max reprice count reached ({count}/{MAX_CHASE_REPRICES})")
            }
            Self::Drift { drift_fraction } => format!(
                "price drift limit exceeded ({:.2}% > {:.2}%)",
                drift_fraction * 100.0,
                MAX_CHASE_DRIFT_FRACTION * 100.0
            ),
        }
    }
}

fn chase_account_matches(chase: &ChaseOrder, connected_address: Option<&str>) -> bool {
    connected_address == Some(chase.account_address.as_str())
}

pub(super) fn chase_reprice_limit_reason(
    chase: &ChaseOrder,
    next_price: f64,
    now: Instant,
) -> Option<ChaseLimitReason> {
    if !chase.initial_price.is_finite()
        || chase.initial_price <= 0.0
        || !next_price.is_finite()
        || next_price <= 0.0
    {
        return Some(ChaseLimitReason::InvalidPrice);
    }

    let elapsed = now.saturating_duration_since(chase.started_at);
    if elapsed >= MAX_CHASE_DURATION {
        return Some(ChaseLimitReason::Timeout { elapsed });
    }

    if chase.reprice_count >= MAX_CHASE_REPRICES {
        return Some(ChaseLimitReason::MaxReprices {
            count: chase.reprice_count,
        });
    }

    let drift_fraction = (next_price - chase.initial_price).abs() / chase.initial_price;
    if drift_fraction > MAX_CHASE_DRIFT_FRACTION {
        return Some(ChaseLimitReason::Drift { drift_fraction });
    }

    None
}

pub(super) fn plan_stop_chase(chase: &mut ChaseOrder) -> StopChaseAction {
    chase.stop_requested = true;
    match chase.current_oid {
        Some(oid) if !chase.cancel_in_flight => {
            chase.cancel_in_flight = true;
            StopChaseAction::CancelResting {
                asset: chase.asset,
                oid,
            }
        }
        Some(_) => StopChaseAction::AwaitCancelResult,
        None => StopChaseAction::AwaitPlaceResult,
    }
}

impl TradingTerminal {
    pub(crate) fn stop_chase(&mut self) -> Task<Message> {
        let _theme = self.theme();
        self.pending_order_action = None;
        let Some(chase) = self.active_chase.as_mut() else {
            return Task::none();
        };

        self.order_status = Some(("Chase stopped".to_string(), false));

        match plan_stop_chase(chase) {
            StopChaseAction::CancelResting { asset, oid } => {
                let key = chase.agent_key.trim().to_string();
                if key.is_empty() {
                    self.order_status = Some((
                        "Chase stopped: original agent key is unavailable".into(),
                        true,
                    ));
                    self.active_chase = None;
                    return Task::none();
                }
                self.order_status =
                    Some((format!("Chase stopping: cancelling order {oid}"), false));
                Task::perform(cancel_order(key.into(), asset, oid), |r| {
                    Message::ChaseCancelResult(Box::new(r))
                })
            }
            StopChaseAction::AwaitPlaceResult => {
                self.order_status = Some((
                    "Chase stopping: waiting for order id before cancelling".into(),
                    false,
                ));
                Task::none()
            }
            StopChaseAction::AwaitCancelResult => {
                self.order_status =
                    Some(("Chase stopping: cancel already in flight".into(), false));
                Task::none()
            }
        }
    }

    fn stop_chase_for_limit(&mut self, reason: ChaseLimitReason) -> Task<Message> {
        let detail = reason.status_detail();
        let stop_task = self.stop_chase();
        self.order_status = Some((format!("Chase stopped: {detail}"), true));
        stop_task
    }

    pub(crate) fn stop_chase_if_limits_reached(&mut self, now: Instant) -> Task<Message> {
        let Some(chase) = self.active_chase.as_ref() else {
            return Task::none();
        };
        let Some(reason) = chase_reprice_limit_reason(chase, chase.current_price, now) else {
            return Task::none();
        };
        self.stop_chase_for_limit(reason)
    }

    /// Return the active order book's best bid/ask, falling back to a fixed
    /// book for the same coin when no active book is available.
    pub(crate) fn best_chase_price(&self, coin: &str, is_buy: bool) -> Option<f64> {
        let price_from_book = |book: &OrderBookInstance| {
            if is_buy {
                book.book.bids.first().map(|level| level.px)
            } else {
                book.book.asks.first().map(|level| level.px)
            }
        };

        self.order_books
            .values()
            .find(|book| match &book.mode {
                OrderBookSymbolMode::Active => self.active_symbol == coin,
                OrderBookSymbolMode::Fixed(_) => false,
            })
            .or_else(|| {
                self.order_books.values().find(|book| match &book.mode {
                    OrderBookSymbolMode::Active => false,
                    OrderBookSymbolMode::Fixed(symbol) => symbol == coin,
                })
            })
            .and_then(price_from_book)
            .filter(|price| price.is_finite() && *price > 0.0)
    }

    /// Cancel the current chase order and reprice at the new best bid/ask.
    pub(crate) fn chase_cancel_and_reprice(&mut self) -> Task<Message> {
        let _theme = self.theme();
        if let Some(chase) = self.active_chase.as_ref()
            && !chase_account_matches(chase, self.connected_address.as_deref())
        {
            let stop_task = self.stop_chase();
            self.order_status =
                Some(("Chase stopped: account changed before reprice".into(), true));
            return stop_task;
        }

        let Some(chase_snapshot) = self.active_chase.as_ref() else {
            return Task::none();
        };
        let Some(best) = self.best_chase_price(&chase_snapshot.coin, chase_snapshot.is_buy) else {
            return Task::none();
        };
        if let Some(reason) = chase_reprice_limit_reason(chase_snapshot, best, Instant::now()) {
            return self.stop_chase_for_limit(reason);
        }

        let Some(chase) = &mut self.active_chase else {
            return Task::none();
        };
        let Some(oid) = chase.current_oid else {
            return Task::none();
        };

        chase.cancel_in_flight = true;
        let key = chase.agent_key.trim().to_string();
        let asset = chase.asset;

        Task::perform(cancel_order(key.into(), asset, oid), |r| {
            Message::ChaseCancelResult(Box::new(r))
        })
    }

    /// Place a new chase limit order at the current best bid/ask.
    pub(crate) fn chase_place_at_best(&mut self) -> Task<Message> {
        let _theme = self.theme();
        // Extract coin before mutable borrow to avoid borrow conflict
        let Some(chase_snapshot) = self.active_chase.as_ref() else {
            return Task::none();
        };
        let coin = chase_snapshot.coin.clone();
        let is_buy_snapshot = chase_snapshot.is_buy;
        let key = chase_snapshot.agent_key.trim().to_string();
        if !chase_account_matches(chase_snapshot, self.connected_address.as_deref()) {
            self.order_status = Some((
                "Chase stopped: account changed before replacement".into(),
                true,
            ));
            self.active_chase = None;
            return Task::none();
        }
        let coin_is_spot = self.is_spot_coin(&coin);
        let best_px = self.best_chase_price(&coin, is_buy_snapshot);

        let Some(chase) = &mut self.active_chase else {
            return Task::none();
        };

        if !chase.remaining_size.is_finite() {
            self.order_status = Some(("Chase stopped: invalid remaining size".to_string(), true));
            self.active_chase = None;
            return Task::none();
        }

        if chase.remaining_size <= 0.0 {
            self.order_status = Some(("Chase fully filled".to_string(), false));
            self.active_chase = None;
            return Task::none();
        }

        let Some(best) = best_px else {
            self.order_status = Some(("Chase: no book data to reprice".into(), true));
            self.active_chase = None;
            return Task::none();
        };
        if key.is_empty() {
            self.order_status = Some((
                "Chase stopped: original agent key is unavailable".into(),
                true,
            ));
            self.active_chase = None;
            return Task::none();
        }
        if let Some(reason) = chase_reprice_limit_reason(chase, best, Instant::now()) {
            self.order_status = Some((format!("Chase stopped: {}", reason.status_detail()), true));
            self.active_chase = None;
            return Task::none();
        }

        let price = float_to_wire(round_price(best, chase.sz_decimals, coin_is_spot));
        let size = float_to_wire(chase.remaining_size);
        let asset = chase.asset;
        let is_buy = chase.is_buy;
        let reduce_only = chase.reduce_only;

        chase.current_price = best;
        chase.current_oid = None;
        chase.cancel_in_flight = false;
        chase.oid_confirmed = false;
        chase.reprice_count = chase.reprice_count.saturating_add(1);

        Task::perform(
            place_order(
                key.into(),
                asset,
                is_buy,
                price,
                size,
                OrderKind::Limit,
                reduce_only,
            ),
            |r| Message::ChasePlaceResult(Box::new(r)),
        )
    }

    /// Check whether the chase order's current oid is still in open orders.
    pub(crate) fn chase_order_still_open(&self) -> bool {
        let _theme = self.theme();
        let Some(chase) = &self.active_chase else {
            return false;
        };
        let Some(oid) = chase.current_oid else {
            return false;
        };
        self.account_data
            .as_ref()
            .map(|d| d.open_orders.iter().any(|o| o.oid == oid))
            .unwrap_or(false)
    }
}
