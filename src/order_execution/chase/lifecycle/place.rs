use super::{chase_account_matches, chase_reprice_limit_reason};

use crate::api::{MarketType, OrderBook};
use crate::app_state::TradingTerminal;
use crate::helpers::{positive_finite_value, redact_sensitive_response_text};
use crate::message::Message;
use crate::order_execution::{
    OrderSurface, PreparedExchangeOrder, open_order_matches_chase_identity, place_order_task,
};
use crate::signing::{
    ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason, ExchangeOrderKind,
    chase_place_cloid, float_to_wire,
};

use iced::Task;
use std::time::Instant;

use super::super::super::sizing::quantize_order_size;

// ---------------------------------------------------------------------------
// Chase Placement
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn best_chase_price_from_book(book: &OrderBook, is_buy: bool) -> Option<f64> {
        let price = if is_buy {
            book.bids.first().map(|level| level.px)
        } else {
            book.asks.first().map(|level| level.px)
        };
        price.and_then(positive_finite_value)
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
                let error = redact_sensitive_response_text(&error);
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
        sigfigs: (Option<u8>, Option<u8>),
        source_context: crate::read_data_provider::MarketDataSourceContext,
        book: OrderBook,
    ) -> Task<Message> {
        if !self.market_stream_source_is_current(source_context) {
            return Task::none();
        }
        let Some(chase) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        if chase.coin != coin
            || self.symbol_key_is_hidden(&coin)
            || sigfigs != self.canonical_l2_book_sigfigs(&coin)
        {
            return Task::none();
        }
        let Some(best) = Self::best_chase_price_from_book(&book, chase.is_buy) else {
            return Task::none();
        };
        self.chase_reprice_to_best_price(chase_id, best)
    }

    pub(crate) fn handle_chase_book_lagged(
        &mut self,
        chase_id: u64,
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: crate::read_data_provider::MarketDataSourceContext,
        skipped: u64,
    ) -> Task<Message> {
        if !self.market_stream_source_is_current(source_context) {
            return Task::none();
        }
        if self.symbol_key_is_hidden(&coin) {
            return Task::none();
        }
        if sigfigs != self.canonical_l2_book_sigfigs(&coin) {
            return Task::none();
        }

        let oid = {
            let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
                return Task::none();
            };
            if chase.coin != coin
                || chase.lifecycle.is_stopping()
                || chase.has_pending_op()
                || !chase.lifecycle.is_book_repriceable()
            {
                return Task::none();
            }
            chase.desired_price = None;
            chase.current_oid
        };

        let Some(oid) = oid else {
            return Task::none();
        };
        self.check_chase_order_status(
            chase_id,
            oid,
            format!("Chase paused: market data lagged ({skipped} L2 updates skipped)"),
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
        if chase_snapshot.is_spot
            && !self.chase_spot_symbol_identity_is_current(chase_id, &chase_snapshot.coin)
        {
            if let Some(oid) = chase_snapshot.current_oid {
                return self.cancel_known_chase_order_for_safety(
                    chase_id,
                    oid,
                    "Chase stopped: spot market identity changed",
                    true,
                );
            }
            if chase_snapshot.has_exchange_identifier() {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    };
                }
                self.order_status = Some((
                    "Chase paused: spot market identity changed; checking prior exposure".into(),
                    true,
                ));
                return Task::none();
            }
            self.order_status = Some(("Chase stopped: spot market identity changed".into(), true));
            self.remove_chase_order(chase_id);
            return Task::none();
        }
        if chase_snapshot.is_spot
            && let Err(message) =
                self.validate_spot_quantity_denomination(&chase_snapshot.coin, false)
        {
            if let Some(oid) = chase_snapshot.current_oid {
                return self.cancel_known_chase_order_for_safety(chase_id, oid, message, true);
            }
            self.order_status = Some((message, true));
            self.remove_chase_order(chase_id);
            return Task::none();
        }
        if chase_snapshot.is_spot && self.spot_metadata_degraded {
            if let Some(oid) = chase_snapshot.current_oid {
                return self.cancel_known_chase_order_for_safety(
                    chase_id,
                    oid,
                    "Chase stopped: spot metadata has not been verified",
                    true,
                );
            }
            if chase_snapshot.has_exchange_identifier() {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    };
                }
                self.order_status = Some((
                    "Chase paused: spot metadata has not been verified; checking prior exposure"
                        .into(),
                    true,
                ));
                return Task::none();
            }
            self.order_status = Some((
                "Chase stopped: spot metadata has not been verified".into(),
                true,
            ));
            self.remove_chase_order(chase_id);
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
                    "Chase requires manual check: invalid chase price with previous exchange \
                     exposure"
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
            let Some(data) = self.account_data_for_order_account(&chase_snapshot.account_address)
            else {
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
            if !data.has_complete_open_orders_for_symbol(&chase_snapshot.coin)
                || !data.completeness.fills_complete
            {
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
                .find(|order| open_order_matches_chase_identity(chase_snapshot, order))
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
        if positive_finite_value(chase_snapshot.initial_price).is_some()
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
        if !self.can_progress_chase_automation(now) {
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
        let key = chase.agent_key.clone_for_task();
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
        let account_address = chase.account_address.clone();
        let market_type = if chase.is_spot {
            MarketType::Spot
        } else {
            MarketType::Perp
        };
        let place_attempt = chase.place_attempt_count.saturating_add(1);
        let cloid = chase_place_cloid(
            &chase.account_address,
            chase.id,
            chase.started_at_ms,
            place_attempt,
        );
        chase.current_price = rounded_best;
        chase.current_price_wire = price_wire.clone();
        if positive_finite_value(chase.initial_price).is_none() {
            chase.initial_price = rounded_best;
        }
        chase.place_attempt_count = place_attempt;
        chase.current_cloid = Some(cloid.clone());
        chase.current_oid = None;
        chase.lifecycle = ChaseLifecycle::Placing;
        chase.desired_price = Some(rounded_best);
        self.last_advanced_exchange_request_at = Some(now);

        let prepared = PreparedExchangeOrder {
            surface: OrderSurface::Chase,
            symbol_key: chase.coin.clone(),
            asset,
            is_buy,
            price: price_wire,
            size,
            order_kind: ExchangeOrderKind::Limit,
            reduce_only,
            market_type,
        };
        let request = prepared.place_request_with_existing_cloid(cloid);

        self.invalidate_spot_balances_after_exchange_dispatch(&account_address, market_type);
        place_order_task(key, request, move |r| Message::ChasePlaceResult {
            chase_id,
            place_attempt,
            result: r.into(),
        })
    }
}
