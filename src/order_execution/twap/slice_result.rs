use super::helpers::{
    TwapAccountRefresh, TwapExchangeErrorAction, classify_twap_exchange_error,
    twap_cancel_child_task, twap_place_result_refresh_policy,
};
use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, redact_sensitive_response_text};
use crate::message::Message;
use crate::signing::{ExchangeResponse, float_to_wire};
use crate::twap_state::{
    TWAP_MAX_RETRY_ATTEMPTS, TwapChildStatus, TwapEventKind, TwapOrder, TwapPauseReason,
    TwapPendingOp, TwapStatus, twap_response_fill_summary,
};

use iced::Task;
use std::time::Instant;

// ---------------------------------------------------------------------------
// TWAP Slice Results
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn twap_result_task_with_optional_refresh(
        &mut self,
        refresh_policy: TwapAccountRefresh,
        twap_id: u64,
        secondary_task: Task<Message>,
    ) -> Task<Message> {
        if self.twap_refresh_policy_needs_refresh(refresh_policy, twap_id) {
            Task::batch([
                self.refresh_after_twap_result(refresh_policy, twap_id),
                secondary_task,
            ])
        } else {
            secondary_task
        }
    }

    pub(crate) fn handle_twap_slice_result(
        &mut self,
        twap_id: u64,
        slice_index: u32,
        retry_count: u32,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let mut refresh_policy = twap_place_result_refresh_policy(&result);
        let now = Instant::now();
        // A slice retry reuses both the TWAP ID and child CLOID. The index and
        // retry count together identify the exact dispatch without exposing it.
        let pending = match self
            .twap_orders
            .get(&twap_id)
            .and_then(|twap| twap.pending_op.as_ref())
        {
            Some(TwapPendingOp::Place(slice))
                if slice.index == slice_index && slice.retry_count == retry_count =>
            {
                slice.clone()
            }
            Some(TwapPendingOp::Place(_)) => return Task::none(),
            _ => return self.refresh_after_twap_result(refresh_policy, twap_id),
        };

        let mut status_update = None;
        let mut cancel_unexpected = None;
        let mut status_check_cloid = None;
        let mut finish_attempt = true;
        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            twap.pending_op = None;
            match result {
                Ok(response) => {
                    let summary_text = response.summary();
                    let fill_summary = twap_response_fill_summary(&response);
                    let oid = fill_summary.oid.or_else(|| response.order_oid());

                    if let Some(child) = twap.child_order_mut(pending.index) {
                        child.oid = oid;
                        child.exchange_summary = summary_text.clone();
                        child.filled_size = child.filled_size.max(fill_summary.filled_size);
                        child.avg_price = fill_summary.avg_price.or(child.avg_price);
                        child.cloid = Some(pending.cloid.clone());
                    }

                    if response.is_ioc_no_match() {
                        if let Some(child) = twap.child_order_mut(pending.index) {
                            child.status = TwapChildStatus::NoFill;
                        }
                        twap.push_event(
                            TwapEventKind::Placed,
                            format!(
                                "Slice {} did not fill: book moved before the IOC could match",
                                pending.index
                            ),
                            false,
                        );
                        status_update = Some((
                            format!("TWAP slice {} did not fill; continuing", pending.index),
                            false,
                        ));
                        twap.retry_slice = None;
                    } else if response.is_error() {
                        match classify_twap_exchange_error(&summary_text) {
                            TwapExchangeErrorAction::Retry(reason) => {
                                refresh_policy = TwapAccountRefresh::None;
                                let retry_count = pending.retry_count.saturating_add(1);
                                if twap.stop_requested {
                                    twap.retry_slice = None;
                                    if let Some(child) = twap.child_order_mut(pending.index) {
                                        child.status = TwapChildStatus::NoFill;
                                        child.retry_count = retry_count;
                                        child.exchange_summary = summary_text.clone();
                                    }
                                } else if retry_count > TWAP_MAX_RETRY_ATTEMPTS {
                                    finish_attempt = false;
                                    if let Some(child) = twap.child_order_mut(pending.index) {
                                        child.status = TwapChildStatus::Rejected;
                                        child.retry_count = retry_count;
                                    }
                                    twap.status = TwapStatus::Error;
                                    twap.push_event(
                                        TwapEventKind::Error,
                                        format!(
                                            "Slice {} stopped after {} retry attempts: {}",
                                            pending.index, TWAP_MAX_RETRY_ATTEMPTS, summary_text
                                        ),
                                        true,
                                    );
                                    status_update = Some((
                                        format!(
                                            "TWAP slice {} stopped after retry budget: {}",
                                            pending.index, summary_text
                                        ),
                                        true,
                                    ));
                                } else {
                                    finish_attempt = false;
                                    let mut retry_slice = pending.clone();
                                    retry_slice.retry_count = retry_count;
                                    twap.retry_slice = Some(retry_slice);
                                    if let Some(child) = twap.child_order_mut(pending.index) {
                                        child.status = TwapChildStatus::Retrying;
                                        child.retry_count = retry_count;
                                        child.exchange_summary = summary_text.clone();
                                    }
                                    let delay = TwapOrder::retry_delay(retry_count);
                                    twap.pause(
                                        reason,
                                        Some(now + delay),
                                        format!(
                                            "Slice {} paused: {}; retry {}/{} in about {}s",
                                            pending.index,
                                            reason.label(),
                                            retry_count,
                                            TWAP_MAX_RETRY_ATTEMPTS,
                                            delay.as_secs()
                                        ),
                                        true,
                                    );
                                    status_update = Some((
                                        format!(
                                            "TWAP paused: {}; retry {}/{} in about {}s",
                                            reason.label(),
                                            retry_count,
                                            TWAP_MAX_RETRY_ATTEMPTS,
                                            delay.as_secs()
                                        ),
                                        true,
                                    ));
                                }
                            }
                            TwapExchangeErrorAction::Terminal => {
                                finish_attempt = false;
                                if let Some(child) = twap.child_order_mut(pending.index) {
                                    child.status = TwapChildStatus::Rejected;
                                }
                                twap.status = TwapStatus::Error;
                                twap.push_event(
                                    TwapEventKind::Rejected,
                                    format!("Slice {} rejected: {summary_text}", pending.index),
                                    true,
                                );
                                status_update = Some((
                                    format!(
                                        "TWAP stopped: slice {} rejected: {summary_text}",
                                        pending.index
                                    ),
                                    true,
                                ));
                            }
                            TwapExchangeErrorAction::ConsumeSlice => {
                                if let Some(child) = twap.child_order_mut(pending.index) {
                                    child.status = TwapChildStatus::Rejected;
                                }
                                twap.push_event(
                                    TwapEventKind::Rejected,
                                    format!("Slice {} rejected: {summary_text}", pending.index),
                                    true,
                                );
                                status_update = Some((
                                    format!(
                                        "TWAP slice {} rejected: {summary_text}",
                                        pending.index
                                    ),
                                    true,
                                ));
                                twap.retry_slice = None;
                            }
                        }
                    } else if fill_summary.filled_size > 0.0 {
                        let filled_size = fill_summary.filled_size;
                        if let Some(child) = twap.child_order_mut(pending.index) {
                            child.status = TwapChildStatus::Filled;
                            child.filled_size = child.filled_size.max(filled_size);
                        }
                        twap.mark_filled(filled_size);
                        twap.push_event(
                            TwapEventKind::Filled,
                            format!(
                                "Slice {} filled {} @ {}",
                                pending.index,
                                float_to_wire(filled_size),
                                fill_summary
                                    .avg_price
                                    .map(format_price)
                                    .unwrap_or_else(|| format_price(pending.limit_price))
                            ),
                            false,
                        );
                        status_update = Some((
                            format!(
                                "TWAP slice {} filled {} {}",
                                pending.index,
                                float_to_wire(filled_size),
                                twap.display_coin
                            ),
                            false,
                        ));
                        twap.retry_slice = None;
                    } else if response.is_fully_filled() {
                        finish_attempt = false;
                        if let Some(child) = twap.child_order_mut(pending.index) {
                            child.status = TwapChildStatus::AwaitingReconciliation;
                        }
                        twap.status_check_cloid = Some(pending.cloid.clone());
                        twap.pause(
                            TwapPauseReason::StatusUnknown,
                            None,
                            format!(
                                concat!(
                                    "Slice {} reported filled but fill size was unavailable; ",
                                    "checking status"
                                ),
                                pending.index
                            ),
                            true,
                        );
                        status_update = Some((
                            format!(
                                "TWAP slice {} fill size unknown; refreshing account data",
                                pending.index
                            ),
                            true,
                        ));
                        refresh_policy = TwapAccountRefresh::Immediate;
                        status_check_cloid = Some(pending.cloid.clone());
                    } else if let Some(oid) = oid {
                        if let Some(child) = twap.child_order_mut(pending.index) {
                            child.status = TwapChildStatus::UnexpectedResting;
                        }
                        twap.pending_op = Some(TwapPendingOp::CancelUnexpectedResting {
                            oid: Some(oid),
                            cloid: Some(pending.cloid.clone()),
                        });
                        twap.pause(
                            TwapPauseReason::UnexpectedResting,
                            None,
                            format!(
                                "Slice {} unexpectedly rested as oid {oid}; cancelling",
                                pending.index
                            ),
                            true,
                        );
                        let key = twap.agent_key.clone_for_task();
                        cancel_unexpected =
                            Some((key, twap.asset, Some(oid), Some(pending.cloid.clone())));
                        status_update = Some((
                            format!(
                                "TWAP slice {} unexpectedly rested; cancelling",
                                pending.index
                            ),
                            true,
                        ));
                        finish_attempt = false;
                    } else if response.is_ambiguous_order_result() {
                        finish_attempt = false;
                        if let Some(child) = twap.child_order_mut(pending.index) {
                            child.status = TwapChildStatus::StatusUnknown;
                        }
                        twap.status_check_cloid = Some(pending.cloid.clone());
                        twap.pause(
                            TwapPauseReason::StatusUnknown,
                            None,
                            format!(
                                "Slice {} returned ambiguous order status: {}; checking status",
                                pending.index, summary_text
                            ),
                            true,
                        );
                        status_update = Some((
                            format!(
                                "TWAP slice {} status unknown; refreshing account data",
                                pending.index
                            ),
                            true,
                        ));
                        refresh_policy = TwapAccountRefresh::Immediate;
                        status_check_cloid = Some(pending.cloid.clone());
                    } else {
                        if let Some(child) = twap.child_order_mut(pending.index) {
                            child.status = TwapChildStatus::NoFill;
                        }
                        twap.push_event(
                            TwapEventKind::Placed,
                            format!(
                                "Slice {} completed without fill: {summary_text}",
                                pending.index
                            ),
                            false,
                        );
                        status_update = Some((
                            format!("TWAP slice {} completed without fill", pending.index),
                            false,
                        ));
                        twap.retry_slice = None;
                    }
                }
                Err(error) => {
                    let error = redact_sensitive_response_text(&error);
                    finish_attempt = false;
                    if let Some(child) = twap.child_order_mut(pending.index) {
                        child.status = TwapChildStatus::StatusUnknown;
                        child.exchange_summary = error.clone();
                        child.cloid = Some(pending.cloid.clone());
                    }
                    twap.status_check_cloid = Some(pending.cloid.clone());
                    twap.pause(
                        TwapPauseReason::StatusUnknown,
                        None,
                        format!(
                            "Slice {} status unknown after transport error: {}; checking status",
                            pending.index, error
                        ),
                        true,
                    );
                    status_update = Some((
                        format!("TWAP slice {} status unknown: {error}", pending.index),
                        true,
                    ));
                    refresh_policy = TwapAccountRefresh::Immediate;
                    status_check_cloid = Some(pending.cloid.clone());
                }
            }
        }

        if let Some((status, is_error)) = status_update {
            if is_error {
                // TWAPs run unattended; failures need the toast/sound path,
                // not just the closable order ticket pane.
                self.set_order_status(status, true);
            } else {
                self.order_status = Some((status, false));
            }
        }

        if let Some((key, asset, oid, cloid)) = cancel_unexpected {
            if key.is_empty() {
                if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
                    twap.status = TwapStatus::Error;
                    twap.push_event(
                        TwapEventKind::Error,
                        concat!(
                            "Unexpected resting order could not be cancelled: ",
                            "original agent key unavailable"
                        )
                        .to_string(),
                        true,
                    );
                }
                self.order_status = Some((
                    "TWAP error: unexpected resting order could not be cancelled".into(),
                    true,
                ));
                return self.refresh_after_twap_result(TwapAccountRefresh::Immediate, twap_id);
            }
            self.invalidate_spot_balances_after_twap_dispatch(twap_id);
            let cancel_task = twap_cancel_child_task(twap_id, key, asset, oid, cloid);
            return self.twap_result_task_with_optional_refresh(
                refresh_policy,
                twap_id,
                cancel_task,
            );
        }

        if let Some(cloid) = status_check_cloid {
            let status_task = self.check_twap_child_status(twap_id, cloid);
            return self.twap_result_task_with_optional_refresh(
                refresh_policy,
                twap_id,
                status_task,
            );
        }

        if finish_attempt {
            self.finish_twap_attempt(twap_id, now);
        }
        self.archive_twap_if_terminal(twap_id);
        self.refresh_after_twap_result(refresh_policy, twap_id)
    }
}
