use super::{ChaseLimitReason, chase_account_matches, chase_reprice_limit_reason};

use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::positive_finite_value;
use crate::message::Message;
use crate::order_execution::{OrderSurface, PreparedModifyOrder, modify_order_task};
use crate::signing::{
    ChaseLifecycle, ChaseQueuedAction, ChaseStopPhase, ChaseVerificationReason, float_to_wire,
};

use iced::Task;
use std::time::Instant;

use super::super::super::sizing::quantize_order_size;

mod tick;

// ---------------------------------------------------------------------------
// Chase Repricing
// ---------------------------------------------------------------------------

impl TradingTerminal {
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
            return self.stop_chase_by_id_with_reason(
                chase_id,
                "Chase stopped: spot market identity changed",
                true,
            );
        }
        if chase_snapshot.is_spot
            && let Err(message) =
                self.validate_spot_quantity_denomination(&chase_snapshot.coin, false)
        {
            if let Some(oid) = chase_snapshot.current_oid {
                return self.cancel_known_chase_order_for_safety(chase_id, oid, message, true);
            }
            return self.stop_chase_by_id_with_reason(chase_id, message, true);
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
            return self.stop_chase_by_id_with_reason(
                chase_id,
                "Chase stopped: spot metadata has not been verified",
                true,
            );
        }
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
        // Account reconciliation is asynchronous. Spot metadata can refresh
        // while it is in flight, so repeat every market-identity gate at the
        // final exchange-dispatch boundary instead of relying on the earlier
        // book-update check.
        let Some((is_spot, coin, current_oid)) = self
            .chase_orders
            .get(&chase_id)
            .map(|chase| (chase.is_spot, chase.coin.clone(), chase.current_oid))
        else {
            return Task::none();
        };
        if is_spot && !self.chase_spot_symbol_identity_is_current(chase_id, &coin) {
            if let Some(oid) = current_oid {
                return self.cancel_known_chase_order_for_safety(
                    chase_id,
                    oid,
                    "Chase stopped: spot market identity changed",
                    true,
                );
            }
            return self.stop_chase_by_id_with_reason(
                chase_id,
                "Chase stopped: spot market identity changed",
                true,
            );
        }
        if is_spot && let Err(message) = self.validate_spot_quantity_denomination(&coin, false) {
            if let Some(oid) = current_oid {
                return self.cancel_known_chase_order_for_safety(chase_id, oid, message, true);
            }
            return self.stop_chase_by_id_with_reason(chase_id, message, true);
        }
        if is_spot && self.spot_metadata_degraded {
            if let Some(oid) = current_oid {
                return self.cancel_known_chase_order_for_safety(
                    chase_id,
                    oid,
                    "Chase stopped: spot metadata has not been verified",
                    true,
                );
            }
            return self.stop_chase_by_id_with_reason(
                chase_id,
                "Chase stopped: spot metadata has not been verified",
                true,
            );
        }

        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        let Some(oid) = chase.current_oid else {
            return Task::none();
        };
        let residual_size = chase.residual_size();
        let size_source = if let Some(remaining_size) = positive_finite_value(chase.remaining_size)
        {
            remaining_size.min(residual_size)
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
        let key = chase.agent_key.clone_for_task();
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
        let account_address = chase.account_address.clone();
        let market_type = if chase.is_spot {
            MarketType::Spot
        } else {
            MarketType::Perp
        };
        let size = float_to_wire(remaining_size);
        let prepared = PreparedModifyOrder {
            surface: OrderSurface::Chase,
            symbol_key: chase.coin.clone(),
            oid,
            asset,
            is_buy,
            price: price_wire,
            size,
            reduce_only,
            market_type,
        };
        chase.record_oid(oid);
        chase.remaining_size = remaining_size;
        chase.lifecycle = ChaseLifecycle::Modifying { oid };
        chase.last_reprice_at = Some(now);
        chase.desired_price = Some(rounded_best);
        chase.reprice_count = chase.reprice_count.saturating_add(1);
        let reprice_count = chase.reprice_count;
        self.last_advanced_exchange_request_at = Some(now);
        self.order_status = Some((format!("{status} {oid}"), false));

        self.invalidate_spot_balances_after_exchange_dispatch(&account_address, market_type);
        modify_order_task(key, prepared, move |r| Message::ChaseModifyResult {
            chase_id,
            oid,
            reprice_count,
            result: Box::new(r),
        })
    }
}
