use super::super::fills::chase_fill_summary_for_chase;
use super::super::orders::{apply_open_order_to_chase, first_open_chase_oid};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::open_order_matches_chase_identity;
use crate::signing::{
    ChaseLifecycle, ChaseOrder, ChaseQueuedAction, ChaseStopPhase, ChaseVerificationReason,
    MAX_CHASE_CANCEL_RETRIES,
};

use iced::Task;

// ---------------------------------------------------------------------------
// Chase Account Refresh Reconciliation
// ---------------------------------------------------------------------------

fn chase_cancel_retry_cap_status(chase: &ChaseOrder) -> Option<(String, bool)> {
    if chase.cancel_retries < MAX_CHASE_CANCEL_RETRIES {
        return None;
    }

    Some(chase.stop_reason.clone().unwrap_or_else(|| {
        (
            format!(
                concat!(
                    "Chase requires manual check: cancel status could not be confirmed after ",
                    "{} attempts; check open orders"
                ),
                MAX_CHASE_CANCEL_RETRIES
            ),
            true,
        )
    }))
}

impl TradingTerminal {
    pub(crate) fn reconcile_chase_after_account_refresh(&mut self) -> Task<Message> {
        let Some((connected_address, data)) = self.connected_order_account_snapshot() else {
            return Task::none();
        };
        let open_orders = data.open_orders.clone();
        let fills = data.fills.clone();
        let open_orders_complete = data.completeness.open_orders_complete;
        let fills_complete = data.completeness.fills_complete;
        let mut tasks = Vec::new();
        if fills_complete {
            tasks.push(self.reconcile_chase_fills_from_snapshot(
                &connected_address,
                &fills,
                open_orders_complete.then_some(open_orders.as_slice()),
                true,
            ));
        }
        let chase_ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        let mut remove_ids = Vec::new();
        let mut correction_ids = Vec::new();
        let mut replacement_ids = Vec::new();
        let mut status_check_ids = Vec::new();
        let mut cancel_ids = Vec::new();

        for chase_id in chase_ids {
            let Some(chase_snapshot) = self.chase_orders.get(&chase_id) else {
                continue;
            };
            if connected_address.as_str() != chase_snapshot.account_address.as_str()
                || chase_snapshot.has_pending_op()
            {
                continue;
            }
            if let ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::VerifyingCancel { .. },
            } = chase_snapshot.lifecycle
            {
                if !open_orders_complete || !fills_complete {
                    self.order_status = Some((
                        "Chase stopping: waiting for complete account snapshot before clearing"
                            .into(),
                        true,
                    ));
                    continue;
                }
                if let Some(oid) = first_open_chase_oid(chase_snapshot, &open_orders) {
                    if let Some((summary, is_error)) = chase_cancel_retry_cap_status(chase_snapshot)
                    {
                        self.order_status = Some((summary, is_error));
                        continue;
                    }
                    let (summary, is_error) = chase_snapshot
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    cancel_ids.push((chase_id, oid, summary, is_error));
                } else {
                    let (summary, is_error) = chase_snapshot
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    remove_ids.push((chase_id, summary, is_error));
                }
                continue;
            }
            if !chase_snapshot.needs_account_verification()
                || chase_snapshot.lifecycle.is_stopping()
            {
                continue;
            }
            let verification_reason = match chase_snapshot.lifecycle {
                ChaseLifecycle::Verifying { reason } => reason,
                _ => continue,
            };
            let wants_replacement = chase_snapshot.desired_price.is_some();
            if !open_orders_complete || !fills_complete {
                self.order_status = Some((
                    concat!(
                        "Chase paused: account refresh was incomplete; not mutating until fills ",
                        "and open orders are verified"
                    )
                    .into(),
                    true,
                ));
                continue;
            }
            if chase_snapshot.residual_size() <= f64::EPSILON {
                let status = chase_fill_summary_for_chase(&fills, chase_snapshot)
                    .unwrap_or_else(|| "Chase completed: target size filled".to_string());
                let is_error = chase_snapshot.target_size.is_finite()
                    && chase_snapshot.target_size > 0.0
                    && chase_snapshot.filled_size > chase_snapshot.target_size + f64::EPSILON;
                if let Some(oid) = first_open_chase_oid(chase_snapshot, &open_orders) {
                    cancel_ids.push((chase_id, oid, status, is_error));
                } else {
                    remove_ids.push((chase_id, status, is_error));
                }
                continue;
            }

            let Some(oid) = chase_snapshot.current_oid else {
                if matches!(verification_reason, ChaseVerificationReason::Placement) {
                    self.order_status = Some((
                        concat!(
                            "Chase placement status is still uncertain; waiting for ",
                            "orderStatus before placing another order"
                        )
                        .into(),
                        true,
                    ));
                    continue;
                }
                if matches!(
                    verification_reason,
                    ChaseVerificationReason::MissingOrderResolvedNoFill
                ) && wants_replacement
                {
                    replacement_ids.push(chase_id);
                } else if wants_replacement {
                    self.order_status = Some((
                        "Chase replacement blocked: previous order is still unresolved".into(),
                        true,
                    ));
                }
                continue;
            };

            let order = open_orders.iter().find(|order| {
                order.oid == oid && open_order_matches_chase_identity(chase_snapshot, order)
            });
            match order {
                Some(order) => {
                    let mut stop_after_refresh = None;
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        match apply_open_order_to_chase(chase, order) {
                            Ok(oversized) => {
                                let needs_reconcile = chase.desired_price.is_some()
                                    || oversized
                                    || matches!(
                                        verification_reason,
                                        ChaseVerificationReason::SizeCorrection
                                    );
                                if chase.lifecycle.is_stopping() {
                                    stop_after_refresh = chase
                                        .stop_reason
                                        .clone()
                                        .or_else(|| Some(("Chase stopped".to_string(), false)));
                                } else if needs_reconcile {
                                    correction_ids.push(chase_id);
                                } else {
                                    chase.lifecycle = ChaseLifecycle::Resting;
                                    self.order_status =
                                        Some((format!("Chasing (oid {oid})..."), false));
                                }
                            }
                            Err(()) => {
                                let summary = concat!(
                                    "Chase stopped: account refresh could not verify the ",
                                    "chased order"
                                );
                                self.order_status = Some((summary.into(), true));
                                cancel_ids.push((chase_id, order.oid, summary.to_string(), true));
                            }
                        }
                    }
                    if let Some((reason, is_error)) = stop_after_refresh {
                        tasks.push(self.stop_chase_by_id_with_reason(chase_id, reason, is_error));
                    }
                }
                None if open_orders_complete
                    && wants_replacement
                    && matches!(
                        verification_reason,
                        ChaseVerificationReason::MissingOrderResolvedNoFill
                    ) =>
                {
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        chase.record_oid(oid);
                        chase.current_oid = None;
                        chase.lifecycle = ChaseLifecycle::Queued {
                            action: ChaseQueuedAction::Place,
                        };
                    }
                    replacement_ids.push(chase_id);
                }
                None if open_orders_complete && wants_replacement => {
                    status_check_ids.push((chase_id, oid));
                }
                None if open_orders_complete => {
                    if matches!(
                        verification_reason,
                        ChaseVerificationReason::MissingOrderResolvedNoFill
                    ) {
                        let status = chase_fill_summary_for_chase(&fills, chase_snapshot)
                            .unwrap_or_else(|| "Chase ended: order no longer open".to_string());
                        self.order_status = Some((status.clone(), false));
                        remove_ids.push((chase_id, status, false));
                    } else {
                        status_check_ids.push((chase_id, oid));
                    }
                }
                None => {
                    self.order_status = Some((
                        "Chase status uncertain: open orders refresh was incomplete".into(),
                        true,
                    ));
                }
            }
        }

        for (chase_id, summary, is_error) in remove_ids {
            self.order_status = Some((summary.clone(), is_error));
            self.remove_chase_order_with_summary(chase_id, Some(summary));
        }
        for (chase_id, oid, summary, is_error) in cancel_ids {
            tasks.push(self.cancel_known_chase_order_for_safety(chase_id, oid, summary, is_error));
        }
        tasks.extend(status_check_ids.into_iter().map(|(chase_id, oid)| {
            self.check_chase_order_status(
                chase_id,
                oid,
                "Chase checking order status before replacement",
            )
        }));
        tasks.extend(
            correction_ids
                .into_iter()
                .map(|chase_id| self.chase_modify_for_current_price_reconciliation(chase_id)),
        );
        let replacements: Vec<_> = replacement_ids
            .into_iter()
            .filter_map(|chase_id| {
                self.chase_orders
                    .get(&chase_id)
                    .and_then(|chase| chase.desired_price)
                    .map(|best| (chase_id, best))
            })
            .collect();
        tasks.extend(
            replacements
                .into_iter()
                .map(|(chase_id, best)| self.chase_place_at_best(chase_id, best)),
        );
        Task::batch(tasks)
    }
}
