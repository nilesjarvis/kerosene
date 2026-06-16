use super::super::orders::apply_open_order_to_chase;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::open_order_matches_chase_identity;
use crate::signing::{ChaseLifecycle, ChaseVerificationReason};

use iced::Task;

// ---------------------------------------------------------------------------
// Chase Open-Order Disappearance
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::account_update::stream) fn handle_chase_order_disappearance(
        &mut self,
    ) -> Task<Message> {
        let mut needs_refresh = false;
        let Some((_, data)) = self.connected_order_account_snapshot() else {
            return Task::none();
        };
        let open_orders = data.open_orders.clone();
        let chase_ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        let mut cancel_ids = Vec::new();

        for chase_id in chase_ids {
            let Some((oid, lifecycle, has_pending, account_matches)) =
                self.chase_orders.get(&chase_id).map(|chase| {
                    (
                        chase.current_oid,
                        chase.lifecycle,
                        chase.has_pending_op(),
                        self.connected_order_account_matches(&chase.account_address),
                    )
                })
            else {
                continue;
            };
            if !account_matches {
                continue;
            }
            let Some(oid) = oid else {
                continue;
            };
            if has_pending {
                continue;
            }
            if lifecycle.is_stopping() {
                continue;
            }
            let Some(chase_snapshot) = self.chase_orders.get(&chase_id) else {
                continue;
            };
            match open_orders.iter().find(|order| {
                order.oid == oid && open_order_matches_chase_identity(chase_snapshot, order)
            }) {
                Some(order) => {
                    let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
                        continue;
                    };
                    match apply_open_order_to_chase(chase, order) {
                        Ok(oversized) => {
                            if oversized {
                                chase.lifecycle = ChaseLifecycle::Verifying {
                                    reason: ChaseVerificationReason::SizeCorrection,
                                };
                                self.order_status = Some((
                                    "Chase verifying fills before correcting remaining size".into(),
                                    false,
                                ));
                                needs_refresh = true;
                            } else if matches!(lifecycle, ChaseLifecycle::Resting)
                                && !chase.lifecycle.is_stopping()
                            {
                                self.order_status =
                                    Some((format!("Chasing (oid {oid})..."), false));
                            }
                        }
                        Err(()) => {
                            let summary = concat!(
                                "Chase stopped: open-orders stream could not verify the ",
                                "chased order"
                            );
                            self.order_status = Some((summary.into(), true));
                            cancel_ids.push((chase_id, oid, summary.to_string(), true));
                        }
                    }
                }
                None => {
                    if matches!(lifecycle, ChaseLifecycle::Resting) {
                        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                            chase.lifecycle = ChaseLifecycle::Verifying {
                                reason: ChaseVerificationReason::MissingOrder,
                            };
                        }
                        self.order_status = Some((
                            concat!(
                                "Chase checking order status: open-orders stream no longer ",
                                "shows the order"
                            )
                            .into(),
                            false,
                        ));
                        needs_refresh = true;
                    }
                }
            }
        }

        let mut tasks = Vec::new();
        for (chase_id, oid, summary, is_error) in cancel_ids {
            tasks.push(self.cancel_known_chase_order_for_safety(chase_id, oid, summary, is_error));
        }
        if needs_refresh {
            tasks.push(self.refresh_account_data());
        }
        Task::batch(tasks)
    }
}
