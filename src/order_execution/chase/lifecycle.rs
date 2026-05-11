use crate::app_state::TradingTerminal;
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
use crate::message::Message;
use crate::signing::{
    ChaseOrder, ChasePendingOp, MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES,
    OrderKind, cancel_order, float_to_wire, modify_order, place_order,
};
use iced::Task;
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StopChaseAction {
    CancelResting { chase_id: u64, asset: u32, oid: u64 },
    AwaitPlaceResult,
    AwaitModifyResult,
    AwaitCancelResult,
    Clear,
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

#[cfg(test)]
pub(super) fn plan_stop_chase(chase: &mut ChaseOrder) -> StopChaseAction {
    plan_stop_chase_with_reason(chase, "Chase stopped".to_string(), false)
}

fn plan_stop_chase_with_reason(
    chase: &mut ChaseOrder,
    reason: String,
    is_error: bool,
) -> StopChaseAction {
    chase.stop_requested = true;
    chase.stop_reason = Some((reason, is_error));
    match chase.pending_op {
        Some(ChasePendingOp::Place) => StopChaseAction::AwaitPlaceResult,
        Some(ChasePendingOp::Modify { .. }) => StopChaseAction::AwaitModifyResult,
        Some(ChasePendingOp::Cancel { .. }) => StopChaseAction::AwaitCancelResult,
        None => match chase.current_oid {
            Some(oid) => {
                chase.pending_op = Some(ChasePendingOp::Cancel { oid });
                StopChaseAction::CancelResting {
                    chase_id: chase.id,
                    asset: chase.asset,
                    oid,
                }
            }
            None => StopChaseAction::Clear,
        },
    }
}

impl TradingTerminal {
    pub(crate) fn next_chase_id(&mut self) -> u64 {
        let id = self.next_chase_id;
        self.next_chase_id = self.next_chase_id.checked_add(1).unwrap_or(1);
        id
    }

    pub(crate) fn stop_chase(&mut self) -> Task<Message> {
        self.stop_chase_with_reason("Chase stopped", false)
    }

    pub(crate) fn stop_chase_with_reason(
        &mut self,
        reason: impl Into<String>,
        is_error: bool,
    ) -> Task<Message> {
        let _theme = self.theme();
        self.pending_order_action = None;
        let Some(chase) = self.active_chase.as_mut() else {
            return Task::none();
        };

        let reason = reason.into();
        match plan_stop_chase_with_reason(chase, reason.clone(), is_error) {
            StopChaseAction::CancelResting {
                chase_id,
                asset,
                oid,
            } => {
                let key = chase.agent_key.trim().to_string();
                if key.is_empty() {
                    self.order_status = Some((
                        "Chase stopped: original agent key is unavailable".into(),
                        true,
                    ));
                    self.active_chase = None;
                    return Task::none();
                }
                self.order_status = Some((format!("{reason}: cancelling order {oid}"), is_error));
                Task::perform(cancel_order(key.into(), asset, oid), move |r| {
                    Message::ChaseCancelResult {
                        chase_id,
                        oid,
                        result: Box::new(r),
                    }
                })
            }
            StopChaseAction::AwaitPlaceResult => {
                self.order_status = Some((
                    format!("{reason}: waiting for order id before cancelling"),
                    is_error,
                ));
                Task::none()
            }
            StopChaseAction::AwaitModifyResult => {
                self.order_status = Some((
                    format!("{reason}: waiting for modify result before cancelling"),
                    is_error,
                ));
                Task::none()
            }
            StopChaseAction::AwaitCancelResult => {
                self.order_status = Some((format!("{reason}: cancel already in flight"), is_error));
                Task::none()
            }
            StopChaseAction::Clear => {
                self.order_status = Some((reason, is_error));
                self.active_chase = None;
                Task::none()
            }
        }
    }

    fn stop_chase_for_limit(&mut self, reason: ChaseLimitReason) -> Task<Message> {
        self.stop_chase_with_reason(format!("Chase stopped: {}", reason.status_detail()), true)
    }

    pub(crate) fn stop_chase_if_limits_reached(&mut self, now: Instant) -> Task<Message> {
        let Some(chase) = self.active_chase.as_ref() else {
            return Task::none();
        };
        if chase.stop_requested {
            return Task::none();
        }
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

        let valid_price = |price: &f64| price.is_finite() && *price > 0.0;
        if self.active_symbol == coin
            && let Some(price) = self
                .order_books
                .values()
                .filter(|book| book.mode == OrderBookSymbolMode::Active)
                .filter_map(price_from_book)
                .find(valid_price)
        {
            return Some(price);
        }

        self.order_books
            .values()
            .filter(
                |book| matches!(&book.mode, OrderBookSymbolMode::Fixed(symbol) if symbol == coin),
            )
            .filter_map(price_from_book)
            .find(valid_price)
    }

    pub(crate) fn chase_reprice_to_best_price(&mut self, best: f64) -> Task<Message> {
        let _theme = self.theme();
        if let Some(chase) = self.active_chase.as_ref()
            && !chase_account_matches(chase, self.connected_address.as_deref())
        {
            return self
                .stop_chase_with_reason("Chase stopped: account changed before reprice", true);
        }

        let now = Instant::now();
        let Some(chase_snapshot) = self.active_chase.as_ref() else {
            return Task::none();
        };
        if chase_snapshot.stop_requested || chase_snapshot.has_pending_op() {
            return Task::none();
        }
        if !chase_snapshot.can_reprice_now(now) {
            return Task::none();
        }
        let Some((rounded_best, price_wire)) = chase_snapshot.rounded_price(best) else {
            return self.stop_chase_for_limit(ChaseLimitReason::InvalidPrice);
        };
        if price_wire == chase_snapshot.current_price_wire {
            return Task::none();
        }
        if !chase_snapshot.price_moves_toward_fill(rounded_best) {
            return Task::none();
        }
        if let Some(reason) = chase_reprice_limit_reason(chase_snapshot, rounded_best, now) {
            return self.stop_chase_for_limit(reason);
        }

        let Some(chase) = &mut self.active_chase else {
            return Task::none();
        };
        let Some(oid) = chase.current_oid else {
            return Task::none();
        };
        if !chase.remaining_size.is_finite() || chase.remaining_size <= 0.0 {
            self.order_status = Some(("Chase stopped: invalid remaining size".to_string(), true));
            self.active_chase = None;
            return Task::none();
        }
        let key = chase.agent_key.trim().to_string();
        if key.is_empty() {
            self.order_status = Some((
                "Chase stopped: original agent key is unavailable".into(),
                true,
            ));
            self.active_chase = None;
            return Task::none();
        }

        let chase_id = chase.id;
        let asset = chase.asset;
        let is_buy = chase.is_buy;
        let size = float_to_wire(chase.remaining_size);
        let reduce_only = chase.reduce_only;
        chase.pending_op = Some(ChasePendingOp::Modify { oid });
        chase.last_reprice_at = Some(now);
        chase.reprice_count = chase.reprice_count.saturating_add(1);

        Task::perform(
            modify_order(
                key.into(),
                oid,
                asset,
                is_buy,
                price_wire.clone(),
                size,
                reduce_only,
            ),
            move |r| Message::ChaseModifyResult {
                chase_id,
                oid,
                requested_price: rounded_best,
                requested_price_wire: price_wire.clone(),
                result: Box::new(r),
            },
        )
    }

    /// Place a new chase limit order at the current best bid/ask.
    pub(crate) fn chase_place_at_best(&mut self) -> Task<Message> {
        let _theme = self.theme();
        let Some(chase_snapshot) = self.active_chase.as_ref() else {
            return Task::none();
        };
        if !chase_account_matches(chase_snapshot, self.connected_address.as_deref()) {
            self.active_chase = None;
            self.order_status = Some((
                "Chase stopped: account changed before initial placement".into(),
                true,
            ));
            return Task::none();
        }
        let Some(best) = self.best_chase_price(&chase_snapshot.coin, chase_snapshot.is_buy) else {
            self.order_status = Some(("Chase: no book data to place".into(), true));
            self.active_chase = None;
            return Task::none();
        };
        let Some((rounded_best, price_wire)) = chase_snapshot.rounded_price(best) else {
            self.order_status = Some(("Chase stopped: invalid chase price".into(), true));
            self.active_chase = None;
            return Task::none();
        };
        if let Some(reason) =
            chase_reprice_limit_reason(chase_snapshot, rounded_best, Instant::now())
        {
            self.order_status = Some((format!("Chase stopped: {}", reason.status_detail()), true));
            self.active_chase = None;
            return Task::none();
        }

        let Some(chase) = &mut self.active_chase else {
            return Task::none();
        };
        if !chase.remaining_size.is_finite() || chase.remaining_size <= 0.0 {
            self.order_status = Some(("Chase stopped: invalid remaining size".to_string(), true));
            self.active_chase = None;
            return Task::none();
        }
        let key = chase.agent_key.trim().to_string();
        if key.is_empty() {
            self.order_status = Some((
                "Chase stopped: original agent key is unavailable".into(),
                true,
            ));
            self.active_chase = None;
            return Task::none();
        }

        let chase_id = chase.id;
        let size = float_to_wire(chase.remaining_size);
        let asset = chase.asset;
        let is_buy = chase.is_buy;
        let reduce_only = chase.reduce_only;
        chase.current_price = rounded_best;
        chase.current_price_wire = price_wire.clone();
        chase.current_oid = None;
        chase.pending_op = Some(ChasePendingOp::Place);
        chase.oid_confirmed = false;
        chase.missing_open_order_refresh_requested = false;

        Task::perform(
            place_order(
                key.into(),
                asset,
                is_buy,
                price_wire,
                size,
                OrderKind::Limit,
                reduce_only,
            ),
            move |r| Message::ChasePlaceResult {
                chase_id,
                result: Box::new(r),
            },
        )
    }
}
