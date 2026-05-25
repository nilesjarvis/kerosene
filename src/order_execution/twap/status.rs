use self::retry::{TwapStatusRetryDecision, next_twap_status_retry};
use super::helpers::{TwapAccountRefresh, twap_cancel_child_task};
use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::twap_state::{
    TWAP_MAX_RETRY_ATTEMPTS, TWAP_RECONCILIATION_TIMEOUT, TwapChildStatus, TwapEventKind,
    TwapPauseReason, TwapPendingOp, TwapStatus,
};

use iced::Task;
use std::time::Instant;

mod retry;
mod tasks;

// ---------------------------------------------------------------------------
// TWAP Status Reconciliation
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_twap_order_status_result(
        &mut self,
        twap_id: u64,
        cloid: String,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let now = Instant::now();
        let mut cancel_unexpected = None;
        let mut refresh = false;
        let mut retry_status_check = None;
        let mut finish_attempt = false;

        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            if twap.status_check_cloid.as_deref() != Some(cloid.as_str()) {
                return Task::none();
            }

            match result {
                Ok(status) if status.is_missing() || status.is_no_fill_terminal() => {
                    twap.status_check_cloid = None;
                    twap.status_check_retries = 0;
                    twap.retry_slice = None;
                    twap.update_child_orders_matching(
                        |child| child.cloid.as_deref() == Some(cloid.as_str()),
                        |child| {
                            child.oid = status.oid.or(child.oid);
                            child.status = if status.is_missing() {
                                TwapChildStatus::NoFill
                            } else {
                                TwapChildStatus::Rejected
                            };
                            child.exchange_summary = status.raw_summary.clone();
                        },
                    );
                    twap.clear_pause();
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        format!("Slice status reconciled: {}", status.raw_summary),
                        status.is_no_fill_terminal(),
                    );
                    self.order_status = Some((
                        format!("TWAP status reconciled: {}", status.raw_summary),
                        false,
                    ));
                    finish_attempt = true;
                }
                Ok(status) if status.is_open() => {
                    twap.status_check_cloid = None;
                    twap.status_check_retries = 0;
                    twap.update_child_orders_matching(
                        |child| child.cloid.as_deref() == Some(cloid.as_str()),
                        |child| {
                            child.oid = status.oid.or(child.oid);
                            child.status = TwapChildStatus::UnexpectedResting;
                            child.exchange_summary = status.raw_summary.clone();
                        },
                    );
                    twap.pending_op = Some(TwapPendingOp::CancelUnexpectedResting {
                        oid: status.oid,
                        cloid: Some(cloid.clone()),
                    });
                    twap.pause(
                        TwapPauseReason::UnexpectedResting,
                        None,
                        format!("Slice unexpectedly open after status check; cancelling {cloid}"),
                        true,
                    );
                    cancel_unexpected = Some((
                        twap.agent_key.trim().to_string(),
                        twap.asset,
                        status.oid,
                        Some(cloid.clone()),
                    ));
                }
                Ok(status) if status.is_filled() => {
                    twap.update_child_orders_matching(
                        |child| child.cloid.as_deref() == Some(cloid.as_str()),
                        |child| {
                            child.oid = status.oid.or(child.oid);
                            child.status = TwapChildStatus::AwaitingReconciliation;
                            child.exchange_summary = status.raw_summary.clone();
                        },
                    );
                    twap.reconciliation_deadline = Some(now + TWAP_RECONCILIATION_TIMEOUT);
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        format!(
                            "Slice {} is filled on exchange; refreshing account fills",
                            cloid
                        ),
                        false,
                    );
                    self.order_status = Some((
                        "TWAP child filled on exchange; refreshing account fills".to_string(),
                        false,
                    ));
                    refresh = true;
                }
                Ok(status) => {
                    let retry = next_twap_status_retry(twap.status_check_retries);
                    twap.status_check_retries = retry.attempt();
                    match retry {
                        TwapStatusRetryDecision::Exhausted { .. } => {
                            twap.status_check_cloid = None;
                            twap.status = TwapStatus::Error;
                            twap.push_event(
                                TwapEventKind::Error,
                                format!(
                                    "Could not reconcile slice {cloid} after status '{}'",
                                    status.status
                                ),
                                true,
                            );
                        }
                        TwapStatusRetryDecision::Retry { attempt, delay } => {
                            twap.pause(
                                TwapPauseReason::StatusUnknown,
                                Some(now + delay),
                                format!(
                                    "Slice status still unclear ({}); retry {}/{} in about {}s",
                                    status.status,
                                    attempt,
                                    TWAP_MAX_RETRY_ATTEMPTS,
                                    delay.as_secs()
                                ),
                                true,
                            );
                            retry_status_check = Some((cloid.clone(), delay));
                        }
                    }
                }
                Err(error) => {
                    let retry = next_twap_status_retry(twap.status_check_retries);
                    twap.status_check_retries = retry.attempt();
                    match retry {
                        TwapStatusRetryDecision::Exhausted { .. } => {
                            twap.status_check_cloid = None;
                            twap.status = TwapStatus::Error;
                            twap.push_event(
                                TwapEventKind::Error,
                                format!(
                                    "Could not check slice status after {} attempts: {error}",
                                    TWAP_MAX_RETRY_ATTEMPTS
                                ),
                                true,
                            );
                        }
                        TwapStatusRetryDecision::Retry { attempt, delay } => {
                            twap.pause(
                                TwapPauseReason::NetworkError,
                                Some(now + delay),
                                format!(
                                    "Slice status check failed; retry {}/{} in about {}s: {error}",
                                    attempt,
                                    TWAP_MAX_RETRY_ATTEMPTS,
                                    delay.as_secs()
                                ),
                                true,
                            );
                            retry_status_check = Some((cloid.clone(), delay));
                        }
                    }
                }
            }
        }

        if let Some((key, asset, oid, cloid)) = cancel_unexpected {
            return twap_cancel_child_task(twap_id, key, asset, oid, cloid);
        }
        if let Some((cloid, delay)) = retry_status_check {
            return self.check_twap_child_status_after(twap_id, cloid, delay);
        }
        if finish_attempt {
            self.finish_twap_attempt(twap_id, now);
        }
        self.archive_twap_if_terminal(twap_id);
        if refresh {
            self.refresh_after_twap_result(TwapAccountRefresh::Immediate, twap_id)
        } else {
            Task::none()
        }
    }
}
