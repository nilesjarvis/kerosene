use super::super::sizing::quantize_order_size;
use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use crate::signing::{
    ChaseLifecycle, ChaseOrder, ChaseQueuedAction, ChaseStopPhase, ChaseVerificationReason,
    MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES, OrderKind,
    PlaceOrderRequest, cancel_order, chase_place_cloid, float_to_wire, modify_order,
    place_order_with_cloid,
};
use crate::twap_state::ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChaseStatusRetry {
    Placement,
    Oid { oid: u64 },
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
    chase.stop_reason = Some((reason, is_error));
    match chase.lifecycle {
        ChaseLifecycle::Placing => {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::AwaitingPlace,
            };
            StopChaseAction::AwaitPlaceResult
        }
        ChaseLifecycle::Modifying { oid } => {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::AwaitingModify { oid },
            };
            StopChaseAction::AwaitModifyResult
        }
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingPlace,
        } => StopChaseAction::AwaitPlaceResult,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingModify { .. },
        } => StopChaseAction::AwaitModifyResult,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { .. },
        } => StopChaseAction::AwaitCancelResult,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::VerifyingCancel { oid },
        } => {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::Canceling { oid },
            };
            StopChaseAction::CancelResting {
                chase_id: chase.id,
                asset: chase.asset,
                oid,
            }
        }
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement,
        } if chase.current_oid.is_none() => {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::AwaitingPlace,
            };
            StopChaseAction::AwaitPlaceResult
        }
        _ => match chase.current_oid {
            Some(oid) => {
                chase.lifecycle = ChaseLifecycle::Stopping {
                    phase: ChaseStopPhase::Canceling { oid },
                };
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
        let Some(chase_id) = self.selected_chase_id() else {
            return Task::none();
        };
        self.stop_chase_by_id_with_reason(chase_id, "Chase stopped", false)
    }

    pub(crate) fn stop_chase_by_id(&mut self, chase_id: u64) -> Task<Message> {
        self.stop_chase_by_id_with_reason(chase_id, "Chase stopped", false)
    }

    pub(crate) fn stop_chase_by_id_with_reason(
        &mut self,
        chase_id: u64,
        reason: impl Into<String>,
        is_error: bool,
    ) -> Task<Message> {
        let _theme = self.theme();
        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        let clear_startup_pending = matches!(
            self.pending_order_action,
            Some(PendingOrderAction::ChaseBuy | PendingOrderAction::ChaseSell)
        ) && chase.current_oid.is_none()
            && !chase.has_pending_op();
        if clear_startup_pending {
            self.pending_order_action = None;
        }

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
                    self.remove_chase_order(chase_id);
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
                self.order_status = Some((format!("{reason}: modify already in flight"), is_error));
                Task::none()
            }
            StopChaseAction::AwaitCancelResult => {
                self.order_status = Some((format!("{reason}: cancel already in flight"), is_error));
                Task::none()
            }
            StopChaseAction::Clear => {
                self.order_status = Some((reason, is_error));
                self.remove_chase_order(chase_id);
                Task::none()
            }
        }
    }

    pub(crate) fn stop_all_chases(&mut self) -> Task<Message> {
        let ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        Task::batch(
            ids.into_iter()
                .map(|id| self.stop_chase_by_id_with_reason(id, "Chase stopped", false)),
        )
    }

    fn stop_chase_for_limit(&mut self, chase_id: u64, reason: ChaseLimitReason) -> Task<Message> {
        self.stop_chase_by_id_with_reason(
            chase_id,
            format!("Chase stopped: {}", reason.status_detail()),
            true,
        )
    }

    pub(crate) fn stop_chase_if_limits_reached(&mut self, now: Instant) -> Task<Message> {
        let stops: Vec<_> = self
            .chase_orders
            .iter()
            .filter_map(|(id, chase)| {
                if chase.lifecycle.is_stopping()
                    || !chase.current_price.is_finite()
                    || chase.current_price <= 0.0
                {
                    return None;
                }
                chase_reprice_limit_reason(chase, chase.current_price, now)
                    .map(|reason| (*id, reason))
            })
            .collect();
        let tasks = stops
            .into_iter()
            .map(|(id, reason)| self.stop_chase_for_limit(id, reason));
        Task::batch(tasks)
    }

    pub(crate) fn cancel_known_chase_order_for_safety(
        &mut self,
        chase_id: u64,
        oid: u64,
        reason: impl Into<String>,
        is_error: bool,
    ) -> Task<Message> {
        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        let key = chase.agent_key.trim().to_string();
        let reason = reason.into();
        chase.record_oid(oid);
        chase.current_oid = Some(oid);
        chase.lifecycle = ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid },
        };
        chase.stop_reason = Some((reason.clone(), is_error));
        if key.is_empty() {
            self.order_status = Some((
                format!("{reason}: manual check required; original agent key is unavailable"),
                true,
            ));
            return Task::none();
        }

        let asset = chase.asset;
        self.order_status = Some((format!("{reason}: cancelling order {oid}"), is_error));
        Task::perform(cancel_order(key.into(), asset, oid), move |r| {
            Message::ChaseCancelResult {
                chase_id,
                oid,
                result: Box::new(r),
            }
        })
    }

    pub(crate) fn retry_stopped_chase_cancels(&mut self, now: Instant) -> Task<Message> {
        if !self.can_send_chase_exchange_request(now) {
            return Task::none();
        }
        let Some((chase_id, reason, is_error)) =
            self.chase_orders.iter().find_map(|(id, chase)| {
                if matches!(
                    chase.lifecycle,
                    ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::VerifyingCancel { .. }
                    }
                ) && chase.current_oid.is_some()
                    && chase.cancel_retries > 0
                    && chase.cancel_retries < crate::signing::MAX_CHASE_CANCEL_RETRIES
                    && chase.can_reprice_now(now)
                {
                    let (reason, is_error) = chase
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    Some((*id, reason, is_error))
                } else {
                    None
                }
            })
        else {
            return Task::none();
        };

        self.stop_chase_by_id_with_reason(chase_id, reason, is_error)
    }

    fn can_send_chase_exchange_request(&self, now: Instant) -> bool {
        !self.account_loading
            && !self.account_reconciliation_required
            && self.last_advanced_exchange_request_at.is_none_or(|last| {
                now.saturating_duration_since(last) >= ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL
            })
    }

    fn clear_chase_desired_price(&mut self, chase_id: u64) {
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.desired_price = None;
            if matches!(
                chase.lifecycle,
                ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::Reprice
                }
            ) {
                chase.lifecycle = ChaseLifecycle::Resting;
            }
        }
    }

    fn update_verifying_chase_desired_price(
        &mut self,
        chase_id: u64,
        rounded_best: f64,
        price_wire: String,
        now: Instant,
    ) -> Task<Message> {
        let Some(chase_snapshot) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        if !matches!(chase_snapshot.lifecycle, ChaseLifecycle::Verifying { .. }) {
            return Task::none();
        }
        if price_wire == chase_snapshot.current_price_wire {
            self.clear_chase_desired_price(chase_id);
            return Task::none();
        }
        if !chase_snapshot.price_moves_toward_fill(rounded_best) {
            self.clear_chase_desired_price(chase_id);
            return Task::none();
        }
        if let Some(reason) = chase_reprice_limit_reason(chase_snapshot, rounded_best, now) {
            return self.stop_chase_for_limit(chase_id, reason);
        }
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.desired_price = Some(rounded_best);
        }
        Task::none()
    }

    fn best_chase_price_from_book(book: &OrderBook, is_buy: bool) -> Option<f64> {
        let price = if is_buy {
            book.bids.first().map(|level| level.px)
        } else {
            book.asks.first().map(|level| level.px)
        };
        price.filter(|price| price.is_finite() && *price > 0.0)
    }

    pub(crate) fn handle_chase_initial_book_loaded(
        &mut self,
        chase_id: u64,
        result: Result<OrderBook, String>,
    ) -> Task<Message> {
        let Some(chase) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        let is_buy = chase.is_buy;
        match result {
            Ok(book) => {
                let Some(best) = Self::best_chase_price_from_book(&book, is_buy) else {
                    self.order_status = Some(("Chase stopped: no book data to place".into(), true));
                    self.pending_order_action = None;
                    self.remove_chase_order(chase_id);
                    return Task::none();
                };
                self.chase_place_at_best(chase_id, best)
            }
            Err(error) => {
                self.order_status =
                    Some((format!("Chase stopped: book load failed: {error}"), true));
                self.pending_order_action = None;
                self.remove_chase_order(chase_id);
                Task::none()
            }
        }
    }

    pub(crate) fn handle_chase_book_update(
        &mut self,
        chase_id: u64,
        coin: String,
        book: OrderBook,
    ) -> Task<Message> {
        let Some(chase) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        if chase.coin != coin || self.symbol_key_is_hidden(&coin) {
            return Task::none();
        }
        let Some(best) = Self::best_chase_price_from_book(&book, chase.is_buy) else {
            return Task::none();
        };
        self.chase_reprice_to_best_price(chase_id, best)
    }

    pub(crate) fn handle_chase_reprice_tick(&mut self) -> Task<Message> {
        let now = Instant::now();
        if !self.can_send_chase_exchange_request(now) {
            return Task::none();
        }
        let status_retry = self.chase_orders.iter().find_map(|(id, chase)| {
            if !chase.can_reprice_now(now) {
                return None;
            }
            match chase.lifecycle {
                ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Placement,
                }
                | ChaseLifecycle::Stopping {
                    phase: ChaseStopPhase::AwaitingPlace,
                } if chase.current_oid.is_none() && chase.current_cloid.is_some() => {
                    Some((*id, ChaseStatusRetry::Placement))
                }
                ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Modify,
                }
                | ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::MissingOrder,
                } => chase
                    .current_oid
                    .map(|oid| (*id, ChaseStatusRetry::Oid { oid })),
                _ => None,
            }
        });
        if let Some((chase_id, retry)) = status_retry {
            return match retry {
                ChaseStatusRetry::Placement => self.check_chase_place_status_by_cloid(
                    chase_id,
                    "retrying placement status".to_string(),
                ),
                ChaseStatusRetry::Oid { oid } => self.check_chase_order_status(
                    chase_id,
                    oid,
                    "Chase retrying order status check",
                ),
            };
        }
        let next = self.chase_orders.iter().find_map(|(id, chase)| {
            if chase.has_pending_op() {
                return None;
            }
            if !chase.can_reprice_now(now) {
                return None;
            }
            match chase.lifecycle {
                ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::Place,
                } if chase.desired_price.is_some() => Some((*id, ChaseQueuedAction::Place)),
                ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::Reprice,
                } if chase.desired_price.is_some() => Some((*id, ChaseQueuedAction::Reprice)),
                ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::SizeCorrection,
                } => Some((*id, ChaseQueuedAction::SizeCorrection)),
                _ => None,
            }
        });
        let Some((chase_id, action)) = next else {
            return Task::none();
        };
        match action {
            ChaseQueuedAction::Place => {
                let Some(best) = self.chase_orders.get(&chase_id).and_then(|chase| chase.desired_price) else {
                    return Task::none();
                };
                self.chase_place_at_best(chase_id, best)
            }
            ChaseQueuedAction::Reprice => {
                let Some(best) = self.chase_orders.get(&chase_id).and_then(|chase| chase.desired_price) else {
                    return Task::none();
                };
                self.chase_reprice_to_best_price(chase_id, best)
            }
            ChaseQueuedAction::SizeCorrection => {
                self.chase_modify_for_current_price_reconciliation(chase_id)
            }
        }
    }

    pub(crate) fn chase_reprice_to_best_price(
        &mut self,
        chase_id: u64,
        best: f64,
    ) -> Task<Message> {
        let _theme = self.theme();
        if let Some(chase) = self.chase_orders.get(&chase_id)
            && !chase_account_matches(chase, self.connected_address.as_deref())
        {
            return self.stop_chase_by_id_with_reason(
                chase_id,
                "Chase stopped: account changed before reprice",
                true,
            );
        }

        let now = Instant::now();
        let Some(chase_snapshot) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        if chase_snapshot.current_oid.is_none() {
            return Task::none();
        }
        let Some((rounded_best, price_wire)) = chase_snapshot.rounded_price(best) else {
            return self.stop_chase_for_limit(chase_id, ChaseLimitReason::InvalidPrice);
        };
        if !chase_snapshot.lifecycle.is_book_repriceable() || chase_snapshot.has_pending_op() {
            return self.update_verifying_chase_desired_price(
                chase_id,
                rounded_best,
                price_wire,
                now,
            );
        }
        if price_wire == chase_snapshot.current_price_wire {
            self.clear_chase_desired_price(chase_id);
            return Task::none();
        }
        if !chase_snapshot.price_moves_toward_fill(rounded_best) {
            self.clear_chase_desired_price(chase_id);
            return Task::none();
        }
        if !chase_snapshot.can_reprice_now(now) || !self.can_send_chase_exchange_request(now) {
            if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                chase.desired_price = Some(rounded_best);
                chase.lifecycle = ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::Reprice,
                };
            }
            return Task::none();
        }
        if let Some(reason) = chase_reprice_limit_reason(chase_snapshot, rounded_best, now) {
            return self.stop_chase_for_limit(chase_id, reason);
        }

        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.desired_price = Some(rounded_best);
            chase.lifecycle = ChaseLifecycle::Verifying {
                reason: ChaseVerificationReason::Reprice,
            };
        }
        self.order_status = Some((
            "Chase verifying fills and open orders before modifying current order".into(),
            false,
        ));
        self.refresh_account_data()
    }

    pub(crate) fn chase_modify_for_current_price_reconciliation(
        &mut self,
        chase_id: u64,
    ) -> Task<Message> {
        let now = Instant::now();
        let Some(chase_snapshot) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        if chase_snapshot.lifecycle.is_stopping() || chase_snapshot.has_pending_op() {
            return Task::none();
        }
        if !chase_account_matches(chase_snapshot, self.connected_address.as_deref()) {
            return self.stop_chase_by_id_with_reason(
                chase_id,
                "Chase stopped: account changed before reprice",
                true,
            );
        }
        if !chase_snapshot.can_reprice_now(now) || !self.can_send_chase_exchange_request(now) {
            if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                let action = if chase.desired_price.is_some() {
                    ChaseQueuedAction::Reprice
                } else {
                    ChaseQueuedAction::SizeCorrection
                };
                chase.lifecycle = ChaseLifecycle::Queued { action };
            }
            return Task::none();
        }
        let target_price = chase_snapshot
            .desired_price
            .unwrap_or(chase_snapshot.current_price);
        let Some((rounded_price, price_wire)) = chase_snapshot.rounded_price(target_price) else {
            return self.stop_chase_for_limit(chase_id, ChaseLimitReason::InvalidPrice);
        };
        self.start_chase_reprice_modify(
            chase_id,
            rounded_price,
            price_wire,
            now,
            "Chase correcting remaining size: modifying current order",
        )
    }

    fn start_chase_reprice_modify(
        &mut self,
        chase_id: u64,
        rounded_best: f64,
        price_wire: String,
        now: Instant,
        status: &'static str,
    ) -> Task<Message> {
        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        let Some(oid) = chase.current_oid else {
            return Task::none();
        };
        let residual_size = chase.residual_size();
        let size_source = if chase.remaining_size.is_finite() && chase.remaining_size > 0.0 {
            chase.remaining_size.min(residual_size)
        } else {
            residual_size
        };
        let Some(remaining_size) = quantize_order_size(size_source, chase.sz_decimals) else {
            return self.cancel_known_chase_order_for_safety(
                chase_id,
                oid,
                "Chase completed: target size filled",
                false,
            );
        };
        let key = chase.agent_key.trim().to_string();
        if key.is_empty() {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::VerifyingCancel { oid },
            };
            chase.stop_reason = Some((
                "Chase requires manual check: original agent key is unavailable".into(),
                true,
            ));
            self.order_status = chase.stop_reason.clone();
            return Task::none();
        }

        let chase_id = chase.id;
        let asset = chase.asset;
        let is_buy = chase.is_buy;
        let reduce_only = chase.reduce_only;
        let size = float_to_wire(remaining_size);
        chase.record_oid(oid);
        chase.remaining_size = remaining_size;
        chase.lifecycle = ChaseLifecycle::Modifying { oid };
        chase.last_reprice_at = Some(now);
        chase.desired_price = Some(rounded_best);
        chase.reprice_count = chase.reprice_count.saturating_add(1);
        self.last_advanced_exchange_request_at = Some(now);
        self.order_status = Some((format!("{status} {oid}"), false));

        Task::perform(
            modify_order(
                key.into(),
                oid,
                asset,
                is_buy,
                price_wire,
                size,
                reduce_only,
            ),
            move |r| Message::ChaseModifyResult {
                chase_id,
                oid,
                result: Box::new(r),
            },
        )
    }

    /// Place a new chase limit order at the current best bid/ask.
    pub(crate) fn chase_place_at_best(&mut self, chase_id: u64, best: f64) -> Task<Message> {
        let _theme = self.theme();
        let now = Instant::now();
        let Some(chase_snapshot) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        if chase_snapshot.lifecycle.is_stopping() || chase_snapshot.has_pending_op() {
            return Task::none();
        }
        if !chase_account_matches(chase_snapshot, self.connected_address.as_deref()) {
            if chase_snapshot.has_exchange_identifier() {
                self.order_status = Some((
                    "Chase requires manual check: account changed with previous exchange exposure"
                        .into(),
                    true,
                ));
                return Task::none();
            }
            self.remove_chase_order(chase_id);
            self.order_status = Some((
                "Chase stopped: account changed before initial placement".into(),
                true,
            ));
            return Task::none();
        }
        if let Some(oid) = chase_snapshot.current_oid {
            return self.check_chase_order_status(
                chase_id,
                oid,
                "Chase blocked replacement: verifying previous order before placing",
            );
        }
        let Some((rounded_best, price_wire)) = chase_snapshot.rounded_price(best) else {
            if chase_snapshot.has_exchange_identifier() {
                self.order_status = Some((
                    "Chase requires manual check: invalid chase price with previous exchange exposure"
                        .into(),
                    true,
                ));
                return Task::none();
            }
            self.order_status = Some(("Chase stopped: invalid chase price".into(), true));
            self.remove_chase_order(chase_id);
            return Task::none();
        };
        if !chase_snapshot.known_oids.is_empty() {
            let Some(data) = self.account_data.as_ref() else {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.desired_price = Some(rounded_best);
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    };
                }
                self.order_status = Some((
                    "Chase paused: verifying previous chase exposure before placing replacement"
                        .into(),
                    true,
                ));
                return self.refresh_account_data();
            };
            if !data.completeness.open_orders_complete || !data.completeness.fills_complete {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.desired_price = Some(rounded_best);
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    };
                }
                self.order_status = Some((
                    "Chase paused: account snapshot incomplete; not placing replacement".into(),
                    true,
                ));
                return self.refresh_account_data();
            }
            if let Some(oid) = data
                .open_orders
                .iter()
                .find(|order| chase_snapshot.tracks_oid(order.oid))
                .map(|order| order.oid)
            {
                return self.cancel_known_chase_order_for_safety(
                    chase_id,
                    oid,
                    "Chase blocked replacement: previous chase order is still open",
                    true,
                );
            }
        }
        if chase_snapshot.initial_price.is_finite()
            && chase_snapshot.initial_price > 0.0
            && let Some(reason) = chase_reprice_limit_reason(chase_snapshot, rounded_best, now)
        {
            if chase_snapshot.has_exchange_identifier() {
                self.order_status = Some((
                    format!(
                        "Chase requires manual check: {} with previous exchange exposure",
                        reason.status_detail()
                    ),
                    true,
                ));
                return Task::none();
            }
            self.order_status = Some((format!("Chase stopped: {}", reason.status_detail()), true));
            self.remove_chase_order(chase_id);
            return Task::none();
        }
        if !self.can_send_chase_exchange_request(now) {
            if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                chase.desired_price = Some(rounded_best);
                chase.lifecycle = ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::Place,
                };
            }
            return Task::none();
        }

        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        let Some(place_size) = quantize_order_size(chase.residual_size(), chase.sz_decimals) else {
            self.order_status = Some(("Chase completed: target size filled".to_string(), false));
            self.remove_chase_order(chase_id);
            return Task::none();
        };
        let key = chase.agent_key.trim().to_string();
        if key.is_empty() {
            if chase.has_exchange_identifier() {
                chase.lifecycle = ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::MissingOrder,
                };
                chase.stop_reason = Some((
                    "Chase requires manual check: original agent key is unavailable".into(),
                    true,
                ));
                self.order_status = chase.stop_reason.clone();
                return Task::none();
            }
            self.order_status = Some((
                "Chase stopped: original agent key is unavailable".into(),
                true,
            ));
            self.remove_chase_order(chase_id);
            return Task::none();
        }

        let chase_id = chase.id;
        chase.remaining_size = place_size;
        let size = float_to_wire(place_size);
        let asset = chase.asset;
        let is_buy = chase.is_buy;
        let reduce_only = chase.reduce_only;
        let place_attempt = chase.place_attempt_count.saturating_add(1);
        let cloid = chase_place_cloid(
            &chase.account_address,
            chase.id,
            chase.started_at_ms,
            place_attempt,
        );
        chase.current_price = rounded_best;
        chase.current_price_wire = price_wire.clone();
        if !chase.initial_price.is_finite() || chase.initial_price <= 0.0 {
            chase.initial_price = rounded_best;
        }
        chase.place_attempt_count = place_attempt;
        chase.current_cloid = Some(cloid.clone());
        chase.current_oid = None;
        chase.lifecycle = ChaseLifecycle::Placing;
        chase.desired_price = Some(rounded_best);
        self.last_advanced_exchange_request_at = Some(now);

        Task::perform(
            place_order_with_cloid(
                key.into(),
                PlaceOrderRequest {
                    asset,
                    is_buy,
                    price: price_wire,
                    size,
                    order_kind: OrderKind::Limit,
                    reduce_only,
                    cloid: Some(cloid),
                },
            ),
            move |r| Message::ChasePlaceResult {
                chase_id,
                result: Box::new(r),
            },
        )
    }
}
