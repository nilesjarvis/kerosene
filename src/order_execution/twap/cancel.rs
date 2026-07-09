use super::helpers::{
    TwapAccountRefresh, twap_cancel_child_task, twap_cancel_label, twap_cancel_target_matches,
    twap_child_matches_cancel_target, twap_terminal_cancel_error,
};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::signing::ExchangeResponse;
use crate::twap_state::{
    TWAP_MAX_UNEXPECTED_CANCEL_RETRIES, TwapChildStatus, TwapEventKind, TwapOrder, TwapPauseReason,
    TwapPendingOp, TwapStatus,
};

use iced::Task;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// TWAP Unexpected Resting Cancellation
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_twap_unexpected_cancel_retry_due(
        &mut self,
        twap_id: u64,
        oid: Option<u64>,
        cloid: Option<String>,
        attempt: u32,
    ) -> Task<Message> {
        let Some(twap) = self.twap_orders.get(&twap_id) else {
            return Task::none();
        };
        if twap.status.is_terminal() || twap.cancel_retries != attempt {
            return Task::none();
        }
        let matches_pending_cancel = matches!(
            &twap.pending_op,
            Some(TwapPendingOp::CancelUnexpectedResting {
                oid: pending_oid,
                cloid: pending_cloid,
            }) if twap_cancel_target_matches(
                *pending_oid,
                pending_cloid.as_deref(),
                oid,
                cloid.as_deref(),
            )
        );
        if !matches_pending_cancel {
            return Task::none();
        }

        let key = twap.agent_key.clone_for_task();
        let asset = twap.asset;
        self.invalidate_spot_balances_after_twap_dispatch(twap_id);
        twap_cancel_child_task(twap_id, key, asset, oid, cloid)
    }

    pub(crate) fn handle_twap_unexpected_cancel_result(
        &mut self,
        twap_id: u64,
        oid: Option<u64>,
        cloid: Option<String>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let now = Instant::now();
        let mut retry_cancel = None;
        let mut finish_attempt = false;
        let mut matched_cancel_target = false;
        if let Some(twap) = self.twap_orders.get_mut(&twap_id)
            && matches!(
                &twap.pending_op,
                Some(TwapPendingOp::CancelUnexpectedResting {
                    oid: pending_oid,
                    cloid: pending_cloid,
                }) if twap_cancel_target_matches(
                    *pending_oid,
                    pending_cloid.as_deref(),
                    oid,
                    cloid.as_deref(),
                )
            )
        {
            matched_cancel_target = true;
            let exchange_summary = match &result {
                Ok(response) => response.summary(),
                Err(error) => redact_sensitive_response_text(error),
            };
            twap.update_child_orders_matching(
                |child| twap_child_matches_cancel_target(child, oid, cloid.as_deref()),
                |child| {
                    child.exchange_summary = exchange_summary.clone();
                },
            );
            match result {
                Ok(response) if response.is_confirmed_cancel_result() => {
                    finish_attempt = true;
                    twap.pending_op = None;
                    twap.cancel_retries = 0;
                    twap.update_child_orders_matching(
                        |child| twap_child_matches_cancel_target(child, oid, cloid.as_deref()),
                        |child| {
                            child.status = TwapChildStatus::UnexpectedRestingCancelled;
                        },
                    );
                    twap.clear_pause();
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        format!(
                            "Canceled unexpected resting child {}",
                            twap_cancel_label(oid, cloid.as_deref())
                        ),
                        false,
                    );
                }
                Ok(response) => {
                    let summary = response.summary();
                    if twap_terminal_cancel_error(&summary) {
                        finish_attempt = true;
                        twap.pending_op = None;
                        twap.cancel_retries = 0;
                        twap.update_child_orders_matching(
                            |child| twap_child_matches_cancel_target(child, oid, cloid.as_deref()),
                            |child| {
                                child.status = TwapChildStatus::UnexpectedRestingCancelled;
                            },
                        );
                        twap.clear_pause();
                        twap.push_event(
                            TwapEventKind::Reconciled,
                            format!(
                                "Unexpected resting child {} is no longer open: {summary}",
                                twap_cancel_label(oid, cloid.as_deref())
                            ),
                            true,
                        );
                    } else {
                        finish_attempt = false;
                        twap.cancel_retries = twap.cancel_retries.saturating_add(1);
                        if twap.cancel_retries >= TWAP_MAX_UNEXPECTED_CANCEL_RETRIES {
                            twap.pending_op = None;
                            twap.status = TwapStatus::Error;
                            twap.push_event(
                                TwapEventKind::Error,
                                format!(
                                    concat!(
                                        "Failed to cancel unexpected resting child {} after ",
                                        "{} attempts: {}"
                                    ),
                                    twap_cancel_label(oid, cloid.as_deref()),
                                    TWAP_MAX_UNEXPECTED_CANCEL_RETRIES,
                                    summary
                                ),
                                true,
                            );
                        } else {
                            let delay = TwapOrder::retry_delay(twap.cancel_retries);
                            twap.pause(
                                TwapPauseReason::UnexpectedResting,
                                Some(now + delay),
                                format!(
                                    concat!(
                                        "Cancel retry {}/{} for unexpected resting child {} ",
                                        "in about {}s"
                                    ),
                                    twap.cancel_retries,
                                    TWAP_MAX_UNEXPECTED_CANCEL_RETRIES,
                                    twap_cancel_label(oid, cloid.as_deref()),
                                    delay.as_secs()
                                ),
                                true,
                            );
                            retry_cancel = Some((oid, cloid.clone(), twap.cancel_retries, delay));
                        }
                    }
                }
                Err(error) => {
                    let error = redact_sensitive_response_text(&error);
                    finish_attempt = false;
                    twap.cancel_retries = twap.cancel_retries.saturating_add(1);
                    if twap.cancel_retries >= TWAP_MAX_UNEXPECTED_CANCEL_RETRIES {
                        twap.pending_op = None;
                        twap.status = TwapStatus::Error;
                        twap.push_event(
                            TwapEventKind::Error,
                            format!(
                                concat!(
                                    "Cancel status unknown for unexpected child {} after ",
                                    "{} attempts: {}"
                                ),
                                twap_cancel_label(oid, cloid.as_deref()),
                                TWAP_MAX_UNEXPECTED_CANCEL_RETRIES,
                                error
                            ),
                            true,
                        );
                    } else {
                        let delay = TwapOrder::retry_delay(twap.cancel_retries);
                        twap.pause(
                            TwapPauseReason::UnexpectedResting,
                            Some(now + delay),
                            format!(
                                concat!(
                                    "Cancel status unknown for unexpected child {}; ",
                                    "retry {}/{} in about {}s"
                                ),
                                twap_cancel_label(oid, cloid.as_deref()),
                                twap.cancel_retries,
                                TWAP_MAX_UNEXPECTED_CANCEL_RETRIES,
                                delay.as_secs()
                            ),
                            true,
                        );
                        retry_cancel = Some((oid, cloid.clone(), twap.cancel_retries, delay));
                    }
                }
            }
        }

        if let Some((oid, cloid, attempt, delay)) = retry_cancel {
            return Task::batch([
                twap_unexpected_cancel_retry_due_task(twap_id, oid, cloid, attempt, delay),
                self.refresh_after_twap_result(TwapAccountRefresh::Immediate, twap_id),
            ]);
        }
        if !matched_cancel_target {
            return Task::none();
        }
        if finish_attempt {
            self.finish_twap_attempt(twap_id, now);
        }
        self.archive_twap_if_terminal(twap_id);
        self.refresh_after_twap_result(TwapAccountRefresh::Immediate, twap_id)
    }
}

async fn twap_unexpected_cancel_retry_due_after(delay: Duration) {
    tokio::time::sleep(delay).await;
}

fn twap_unexpected_cancel_retry_due_task(
    twap_id: u64,
    oid: Option<u64>,
    cloid: Option<String>,
    attempt: u32,
    delay: Duration,
) -> Task<Message> {
    Task::perform(twap_unexpected_cancel_retry_due_after(delay), move |()| {
        Message::TwapUnexpectedCancelRetryDue {
            twap_id,
            oid,
            cloid,
            attempt,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unexpected_cancel_retry_due_task_waits_for_delay() {
        let result = tokio::time::timeout(
            Duration::from_millis(50),
            twap_unexpected_cancel_retry_due_after(Duration::from_secs(2)),
        )
        .await;

        assert!(result.is_err());
    }
}
