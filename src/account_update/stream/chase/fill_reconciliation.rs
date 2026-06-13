use super::super::fills::{chase_completed_summary, chase_fill_totals_for_chase};
use super::super::orders::first_open_chase_oid;

use crate::account::{OpenOrder, UserFill};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseVerificationReason};

use iced::Task;

// ---------------------------------------------------------------------------
// Chase Fill Reconciliation
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn reconcile_chase_fills_from_account(&mut self) -> Task<Message> {
        let Some((snapshot_account_address, data)) = self.connected_order_account_snapshot() else {
            return Task::none();
        };
        if !data.completeness.fills_complete {
            return Task::none();
        }
        let fills = data.fills.clone();
        let open_orders = data.open_orders.clone();
        let open_orders_complete = data.completeness.open_orders_complete;
        self.reconcile_chase_fills_from_snapshot(
            &snapshot_account_address,
            &fills,
            open_orders_complete.then_some(open_orders.as_slice()),
            false,
        )
    }

    pub(super) fn reconcile_chase_fills_from_snapshot(
        &mut self,
        snapshot_account_address: &str,
        fills: &[UserFill],
        open_orders: Option<&[OpenOrder]>,
        open_orders_authoritative: bool,
    ) -> Task<Message> {
        let chase_ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        let mut remove_ids = Vec::new();
        let mut cancel_ids = Vec::new();
        let mut needs_open_order_refresh = false;
        for chase_id in chase_ids {
            let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
                continue;
            };
            if chase.account_address != snapshot_account_address {
                continue;
            }
            let Some(totals) = chase_fill_totals_for_chase(fills, chase) else {
                continue;
            };
            chase.set_filled_size(totals.filled_size);
            if chase.residual_size() <= f64::EPSILON {
                if chase.has_pending_op() {
                    // An exchange mutation is already in flight for this
                    // chase; forcing a safety cancel now would put two
                    // mutations in flight for the same order. The in-flight
                    // result triggers an account refresh, and the next
                    // reconcile pass closes the chase out.
                    continue;
                }
                let summary = chase_completed_summary(fills, chase, totals.filled_size);
                let is_error = chase.target_size.is_finite()
                    && chase.target_size > 0.0
                    && totals.filled_size > chase.target_size + f64::EPSILON;
                match open_orders {
                    Some(open_orders) => {
                        if let Some(oid) = first_open_chase_oid(chase, open_orders) {
                            cancel_ids.push((chase_id, oid, summary, is_error));
                        } else if open_orders_authoritative {
                            remove_ids.push((chase_id, summary, is_error));
                        } else {
                            chase.lifecycle = ChaseLifecycle::Verifying {
                                reason: ChaseVerificationReason::MissingOrder,
                            };
                            chase.stop_reason = Some((summary.clone(), is_error));
                            needs_open_order_refresh = true;
                            self.order_status = Some((
                                "Chase target filled; refreshing open orders before closing".into(),
                                is_error,
                            ));
                        }
                    }
                    None => {
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::MissingOrder,
                        };
                        chase.stop_reason = Some((summary.clone(), is_error));
                        needs_open_order_refresh = true;
                        self.order_status = Some((
                            "Chase target filled; verifying open orders before closing".into(),
                            is_error,
                        ));
                    }
                }
            }
        }

        for (chase_id, summary, is_error) in remove_ids {
            self.order_status = Some((summary.clone(), is_error));
            self.remove_chase_order_with_summary(chase_id, Some(summary));
        }
        let mut tasks = Vec::new();
        for (chase_id, oid, summary, is_error) in cancel_ids {
            tasks.push(self.cancel_known_chase_order_for_safety(chase_id, oid, summary, is_error));
        }
        if needs_open_order_refresh {
            tasks.push(self.refresh_account_data());
        }
        Task::batch(tasks)
    }
}
