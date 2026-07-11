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

fn unresolved_child_status_is_unknown(twap: &crate::twap_state::TwapOrder, cloid: &str) -> bool {
    twap.child_orders.iter().any(|child| {
        child.cloid.as_deref() == Some(cloid)
            && matches!(
                child.status,
                TwapChildStatus::StatusUnknown
                    | TwapChildStatus::AwaitingReconciliation
                    | TwapChildStatus::AwaitingNoFillConfirmation
            )
    })
}

impl TradingTerminal {
    pub(crate) fn handle_twap_order_status_result(
        &mut self,
        twap_id: u64,
        cloid: String,
        attempt: u32,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let now = Instant::now();
        let mut cancel_unexpected = None;
        let mut refresh = false;
        let mut retry_status_check = None;
        let mut finish_attempt = false;

        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            if twap.status_check_cloid.as_deref() != Some(cloid.as_str())
                || twap.status_check_pending_attempt != Some(attempt)
            {
                return Task::none();
            }
            if twap.status.is_terminal() {
                return Task::none();
            }
            // Claim this exact task result before applying it. Any duplicate
            // or delayed result for the same CLOID/attempt is now stale, even
            // if `status_check_cloid` remains set for account-fill repair.
            twap.status_check_pending_attempt = None;

            match result {
                Ok(status) if status.is_missing() => {
                    let retry = next_twap_status_retry(twap.status_check_retries);
                    twap.status_check_retries = retry.attempt();
                    match retry {
                        TwapStatusRetryDecision::Exhausted { .. } => {
                            let fail_closed = !twap.stop_requested
                                && unresolved_child_status_is_unknown(twap, &cloid);
                            twap.status_check_cloid = None;
                            twap.status_check_retries = 0;
                            twap.retry_slice = None;
                            if fail_closed {
                                twap.update_child_orders_matching(
                                    |child| child.cloid.as_deref() == Some(cloid.as_str()),
                                    |child| {
                                        child.oid = status.oid.or(child.oid);
                                        child.status = TwapChildStatus::StatusUnknown;
                                        child.exchange_summary = status.raw_summary.clone();
                                    },
                                );
                                twap.status = TwapStatus::Error;
                                twap.paused_until = None;
                                let message = format!(
                                    "TWAP stopped: slice status remained missing after retries: {}; check the exchange before restarting",
                                    status.raw_summary
                                );
                                twap.push_event(TwapEventKind::Error, message.clone(), true);
                                self.order_status = Some((message, true));
                            } else {
                                twap.update_child_orders_matching(
                                    |child| child.cloid.as_deref() == Some(cloid.as_str()),
                                    |child| {
                                        child.oid = status.oid.or(child.oid);
                                        child.status = TwapChildStatus::NoFill;
                                        child.exchange_summary = status.raw_summary.clone();
                                    },
                                );
                                twap.clear_pause();
                                twap.push_event(
                                    TwapEventKind::Reconciled,
                                    format!(
                                        "Slice status remained missing after retries: {}",
                                        status.raw_summary
                                    ),
                                    false,
                                );
                                self.order_status = Some((
                                    format!("TWAP status reconciled: {}", status.raw_summary),
                                    false,
                                ));
                                finish_attempt = true;
                            }
                        }
                        TwapStatusRetryDecision::Retry { attempt, delay } => {
                            twap.update_child_orders_matching(
                                |child| child.cloid.as_deref() == Some(cloid.as_str()),
                                |child| {
                                    child.oid = status.oid.or(child.oid);
                                    child.status = TwapChildStatus::StatusUnknown;
                                    child.exchange_summary = status.raw_summary.clone();
                                },
                            );
                            twap.pause(
                                TwapPauseReason::StatusUnknown,
                                Some(now + delay),
                                format!(
                                    "Slice status missing ({}); retry {}/{} in about {}s",
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
                Ok(status) if status.is_definitive_no_fill_terminal() => {
                    twap.status_check_cloid = None;
                    twap.status_check_retries = 0;
                    twap.retry_slice = None;
                    twap.update_child_orders_matching(
                        |child| child.cloid.as_deref() == Some(cloid.as_str()),
                        |child| {
                            child.oid = status.oid.or(child.oid);
                            child.status = TwapChildStatus::Rejected;
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
                Ok(status) if status.is_no_fill_terminal() => {
                    twap.status_check_retries = 0;
                    twap.retry_slice = None;
                    twap.account_reconciliation_retries = 0;
                    twap.update_child_orders_matching(
                        |child| child.cloid.as_deref() == Some(cloid.as_str()),
                        |child| {
                            child.oid = status.oid.or(child.oid);
                            child.status = TwapChildStatus::AwaitingNoFillConfirmation;
                            child.exchange_summary = status.raw_summary.clone();
                        },
                    );
                    twap.reconciliation_deadline = Some(now + TWAP_RECONCILIATION_TIMEOUT);
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        format!(
                            "Slice status {}; verifying account fills before continuing: {}",
                            status.status, status.raw_summary
                        ),
                        false,
                    );
                    self.order_status = Some((
                        "TWAP child reported no-fill terminal status; verifying account fills"
                            .to_string(),
                        false,
                    ));
                    refresh = true;
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
                    let key = twap.agent_key.clone_for_task();
                    cancel_unexpected = Some((key, twap.asset, status.oid, Some(cloid.clone())));
                }
                Ok(status) if status.is_filled() => {
                    twap.account_reconciliation_retries = 0;
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
            self.invalidate_spot_balances_after_twap_dispatch(twap_id);
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
