use super::super::fills::{chase_completed_summary, chase_fill_totals_for_chase};
use super::super::orders::apply_open_order_to_chase;
use crate::account::{ChaseRepriceSnapshot, fetch_chase_reprice_snapshot};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::open_order_matches_chase_identity;
use crate::read_data_provider::AccountDataRequestContext;
use crate::signing::{ChaseLifecycle, ChaseVerificationReason};

use iced::Task;

impl TradingTerminal {
    /// A narrow pre-modify read is allowed only on top of a healthy full account
    /// snapshot. It never repairs account-wide uncertainty or refreshes balances.
    fn chase_reprice_account_is_ready(&self, address: &str) -> bool {
        let now_ms = Self::now_ms();
        self.connected_order_account_matches(address)
            && self
                .account_data_for_order_account(address)
                .is_some_and(|data| {
                    data.completeness.open_orders_complete
                        && data.completeness.fills_complete
                        && data.completeness.positions_actionable
                        && data.completeness.spot_balances_complete
                        && data.is_fresh_for_position_action(now_ms)
                        && data.is_fresh_for_spot_balance_action(now_ms)
                })
    }

    pub(crate) fn refresh_chase_reprice_data(&mut self, chase_id: u64) -> Task<Message> {
        let Some(chase) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        let Some(oid) = chase.current_oid else {
            return self.refresh_account_data();
        };
        if self.account_loading
            || self.account_reconciliation_required
            || !self.chase_reprice_account_is_ready(&chase.account_address)
            || chase.lifecycle
                != (ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Reprice,
                })
        {
            return self.refresh_account_data();
        }
        let address = chase.account_address.clone();
        let coin = chase.coin.clone();
        let context = self.begin_account_data_request_context();
        let key = self.hydromancer_api_key_for_task();
        self.account_loading = true;
        self.account_reconciliation_required = true;
        Task::perform(
            fetch_chase_reprice_snapshot(address.clone(), coin, self.read_data_provider, key),
            move |result| Message::ChaseRepriceSnapshotLoaded {
                chase_id,
                oid,
                address: address.into(),
                context,
                result: Box::new(result),
            },
        )
    }

    pub(crate) fn handle_chase_reprice_snapshot(
        &mut self,
        chase_id: u64,
        oid: u64,
        address: String,
        context: AccountDataRequestContext,
        result: Result<ChaseRepriceSnapshot, String>,
    ) -> Task<Message> {
        // An older completion must never unlock a newer full refresh, including
        // an away-and-back account switch or a user-stream recovery request.
        if !self.account_data_request_generation_is_current(&address, context)
            || !self.connected_order_account_matches(&address)
            || !self.account_loading
        {
            return Task::none();
        }
        self.account_loading = false;
        let followup = std::mem::take(&mut self.account_refresh_followup_pending);
        let still_verifying = self.chase_orders.get(&chase_id).is_some_and(|chase| {
            chase.account_address == address
                && chase.current_oid == Some(oid)
                && chase.lifecycle
                    == ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::Reprice,
                    }
        });
        if followup
            || !self.read_data_request_context_is_current(context.read_data)
            || !self.chase_reprice_account_is_ready(&address)
            || !still_verifying
        {
            return self.force_refresh_account_data_for_reconciliation(address);
        }
        let Ok(snapshot) = result else {
            // Preserve provider fallback, rate-limit handling, and the existing
            // account error UI through the ordinary full-refresh path.
            return self.force_refresh_account_data_for_reconciliation(address);
        };
        self.account_reconciliation_required = false;
        self.reconcile_chase_reprice_snapshot(chase_id, oid, snapshot)
    }

    fn reconcile_chase_reprice_snapshot(
        &mut self,
        chase_id: u64,
        oid: u64,
        snapshot: ChaseRepriceSnapshot,
    ) -> Task<Message> {
        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        let totals = chase_fill_totals_for_chase(&snapshot.fills, chase);
        if let Some(totals) = &totals {
            chase.set_filled_size(totals.filled_size);
        }
        let order = snapshot
            .open_orders
            .iter()
            .find(|order| order.oid == oid && open_order_matches_chase_identity(chase, order));
        let Some(order) = order else {
            // Absence from a scoped read never authorizes a new order or
            // archives exposure. Use the established status/full-refresh flow.
            return self.check_chase_order_status(
                chase_id,
                oid,
                "Chase checking missing order before repricing",
            );
        };
        if chase.residual_size() <= f64::EPSILON {
            let is_error = chase.filled_size > chase.target_size + f64::EPSILON;
            let coin = chase.coin.clone();
            let target_size = chase.target_size;
            let summary = totals
                .as_ref()
                .map(|totals| {
                    chase_completed_summary(
                        totals,
                        target_size,
                        &self.display_coin_for_journal(&coin),
                    )
                })
                .unwrap_or_else(|| "Chase completed: target size filled".into());
            return self.cancel_known_chase_order_for_safety(chase_id, oid, summary, is_error);
        }
        let Ok(oversized) = apply_open_order_to_chase(chase, order) else {
            return self.cancel_known_chase_order_for_safety(
                chase_id,
                oid,
                "Chase stopped: repricing snapshot could not verify the chased order",
                true,
            );
        };
        if chase.desired_price.is_none() && !oversized {
            chase.lifecycle = ChaseLifecycle::Resting;
            return Task::none();
        }
        self.chase_modify_for_current_price_reconciliation(chase_id)
    }
}
