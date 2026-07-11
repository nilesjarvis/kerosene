use crate::account::UserFill;
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::signing::float_to_wire;
use crate::twap_state::{
    TWAP_MAX_RETRY_ATTEMPTS, TWAP_RECONCILIATION_TIMEOUT, TwapChildOrder, TwapChildStatus,
    TwapEventKind, TwapOrder, TwapStatus,
};

use iced::Task;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// TWAP Account Fill Reconciliation
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_twap_reconciliation_account_data_failure(
        &mut self,
        account_address: &str,
        error: String,
    ) -> Task<Message> {
        let error = redact_sensitive_response_text(&error);
        let now = Instant::now();
        let mut archive_ids = Vec::new();
        let mut retry_delay = None;
        let mut status_update = None;

        for twap in self.twap_orders.values_mut() {
            if twap.account_address != account_address
                || !twap_needs_reconciliation_account_data(twap)
            {
                continue;
            }

            if let Some(_message) = fail_twap_reconciliation_timeout(twap, now) {
                archive_ids.push(twap.id);
                continue;
            }

            let attempt = twap.account_reconciliation_retries.saturating_add(1);
            twap.account_reconciliation_retries = attempt;
            if attempt >= TWAP_MAX_RETRY_ATTEMPTS {
                let message = format!(
                    "TWAP account-fill reconciliation refresh failed after {attempt} attempts: \
                     {error}"
                );
                twap.push_event(TwapEventKind::Error, message.clone(), true);
                status_update = Some(message);
                continue;
            }

            let delay = TwapOrder::retry_delay(attempt);
            retry_delay = Some(shorter_delay(retry_delay, delay));
            let message = format!(
                "TWAP account-fill reconciliation refresh failed; retry {attempt}/{} in about {}s: {error}",
                TWAP_MAX_RETRY_ATTEMPTS,
                delay.as_secs()
            );
            twap.push_event(TwapEventKind::Retrying, message.clone(), true);
            status_update = Some(message);
        }

        for twap_id in archive_ids {
            self.archive_twap_if_terminal(twap_id);
        }

        if let Some(message) = status_update {
            self.set_order_status(message, true);
        }

        let Some(delay) = retry_delay else {
            return Task::none();
        };
        let address = account_address.to_string();
        Task::perform(
            async move {
                tokio::time::sleep(delay).await;
                address
            },
            |address| Message::RetryTwapReconciliationAccountData(address.into()),
        )
    }

    pub(crate) fn twap_reconciliation_account_data_retry_needed(
        &self,
        account_address: &str,
    ) -> bool {
        let now = Instant::now();
        self.twap_orders.values().any(|twap| {
            twap.account_address == account_address
                && twap_needs_reconciliation_account_data(twap)
                && twap.account_reconciliation_retries < TWAP_MAX_RETRY_ATTEMPTS
                && !TwapOrder::reconciliation_timed_out(twap.reconciliation_deadline, now)
        })
    }

    pub(crate) fn expire_twap_reconciliation_timeouts(&mut self, now: Instant) -> bool {
        let mut archive_ids = Vec::new();
        let mut status_update = None;
        for twap in self.twap_orders.values_mut() {
            if let Some(message) = fail_twap_reconciliation_timeout(twap, now) {
                archive_ids.push(twap.id);
                status_update = Some(message);
            }
        }
        let expired = !archive_ids.is_empty();
        for twap_id in archive_ids {
            self.archive_twap_if_terminal(twap_id);
        }
        if let Some(message) = status_update {
            self.set_order_status(message, true);
        }
        expired
    }

    pub(crate) fn reconcile_twap_fills_from_account(&mut self) {
        let Some((address, data)) = self.connected_order_account_snapshot() else {
            return;
        };
        let fills = data.fills.clone();
        self.reconcile_twap_fills_for_account(&address, &fills);
    }

    pub(crate) fn reconcile_twap_fills_for_account(
        &mut self,
        account_address: &str,
        fills: &[UserFill],
    ) {
        self.reconcile_twap_fills_for_account_with_policy(account_address, fills, false);
    }

    pub(crate) fn reconcile_twap_fills_for_account_after_refresh(
        &mut self,
        account_address: &str,
        fills: &[UserFill],
    ) {
        self.reconcile_twap_fills_for_account_with_policy(account_address, fills, true);
    }

    fn reconcile_twap_fills_for_account_with_policy(
        &mut self,
        account_address: &str,
        fills: &[UserFill],
        confirm_no_fill_absence: bool,
    ) {
        let now = Instant::now();
        let mut archive_ids = Vec::new();
        let mut finish_ids = Vec::new();
        let mut status_update = None;
        for twap in self.twap_orders.values_mut() {
            if twap.account_address != account_address {
                continue;
            }
            let before = twap.filled_size;
            let before_status = twap.status;
            let had_reconciliation_child = twap.has_status_unknown_child();
            let no_fill_confirmation_indexes: Vec<u32> = twap
                .child_orders
                .iter()
                .filter(|child| child.status == TwapChildStatus::AwaitingNoFillConfirmation)
                .map(|child| child.index)
                .collect();
            if confirm_no_fill_absence {
                twap.reconcile_fills_confirming_no_fill(fills);
            } else {
                twap.reconcile_fills(fills);
            }
            let filled_size_increased = twap.filled_size > before;
            let no_fill_confirmed = no_fill_confirmation_indexes.iter().any(|index| {
                twap.child_orders
                    .iter()
                    .any(|child| child.index == *index && child.status == TwapChildStatus::NoFill)
            });
            let reconciliation_resolved =
                had_reconciliation_child && !twap.has_status_unknown_child();
            if reconciliation_resolved {
                twap.status_check_cloid = None;
                twap.status_check_pending_attempt = None;
                twap.account_reconciliation_retries = 0;
                twap.reconciliation_deadline = None;
                if before_status == TwapStatus::Paused {
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        "TWAP resumed after account fill reconciliation".to_string(),
                        false,
                    );
                }
                if no_fill_confirmed {
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        "TWAP confirmed no fill after account fill reconciliation".to_string(),
                        false,
                    );
                }
            }
            if filled_size_increased {
                twap.push_event(
                    TwapEventKind::Filled,
                    format!(
                        "Reconciled fills: {} / {} filled",
                        float_to_wire(twap.filled_size),
                        float_to_wire(twap.target_size)
                    ),
                    false,
                );
            } else if !reconciliation_resolved
                && let Some(message) = fail_twap_reconciliation_timeout(twap, now)
            {
                // The exchange reported a child status that cannot be consumed
                // until account fills reconcile it. Tear the TWAP down with a
                // clear error rather than leave it paused indefinitely with
                // `status_check_cloid` set, blocking future slices.
                status_update = Some(message);
            }
            if twap.stop_requested
                && !twap.status.is_terminal()
                && twap.pending_op.is_none()
                && !twap.has_status_unknown_child()
            {
                finish_ids.push(twap.id);
                continue;
            }
            if no_fill_confirmed
                && !twap.status.is_terminal()
                && twap.pending_op.is_none()
                && !twap.has_status_unknown_child()
            {
                finish_ids.push(twap.id);
                continue;
            }
            if twap.status.is_terminal()
                && (twap.filled_size > before || twap.status != before_status)
            {
                archive_ids.push(twap.id);
            }
        }
        for twap_id in finish_ids {
            self.finish_twap_attempt(twap_id, now);
        }
        for twap_id in archive_ids {
            self.archive_twap_if_terminal(twap_id);
        }
        if let Some(message) = status_update {
            self.set_order_status(message, true);
        }
    }
}

fn shorter_delay(current: Option<Duration>, candidate: Duration) -> Duration {
    current.map_or(candidate, |current| current.min(candidate))
}

fn twap_needs_reconciliation_account_data(twap: &TwapOrder) -> bool {
    !twap.status.is_terminal()
        && (twap.status_check_cloid.is_some()
            || twap.reconciliation_deadline.is_some()
            || twap.has_status_unknown_child())
}

fn fail_twap_reconciliation_timeout(twap: &mut TwapOrder, now: Instant) -> Option<String> {
    if !TwapOrder::reconciliation_timed_out(twap.reconciliation_deadline, now)
        || !twap.has_status_unknown_child()
        || twap.status.is_terminal()
    {
        return None;
    }

    let pending_slice = pending_reconciliation_slice_label(twap);
    twap.status_check_cloid = None;
    twap.status_check_pending_attempt = None;
    twap.account_reconciliation_retries = 0;
    twap.reconciliation_deadline = None;
    twap.status = TwapStatus::Error;
    let message = if twap.has_no_fill_confirmation_child() {
        format!(
            "Could not verify {pending_slice} had no fills within {}s; \
             exchange reported a non-definitive no-fill status but account \
             fills did not confirm it. Check the exchange before manually \
             resuming.",
            TWAP_RECONCILIATION_TIMEOUT.as_secs()
        )
    } else {
        format!(
            "Could not reconcile {pending_slice} via account fills within {}s; \
             exchange reported fill but account fills did not catch up. Check the \
             exchange before manually resuming.",
            TWAP_RECONCILIATION_TIMEOUT.as_secs()
        )
    };
    twap.push_event(TwapEventKind::Error, message.clone(), true);
    Some(message)
}

fn pending_reconciliation_slice_label(twap: &TwapOrder) -> String {
    if let Some(cloid) = twap
        .status_check_cloid
        .as_deref()
        .map(str::trim)
        .filter(|cloid| !cloid.is_empty())
    {
        return format!("slice {cloid}");
    }

    twap.child_orders
        .iter()
        .find(|child| {
            matches!(
                child.status,
                TwapChildStatus::StatusUnknown
                    | TwapChildStatus::AwaitingReconciliation
                    | TwapChildStatus::AwaitingNoFillConfirmation
            )
        })
        .map(pending_child_slice_label)
        .unwrap_or_else(|| "unknown slice".to_string())
}

fn pending_child_slice_label(child: &TwapChildOrder) -> String {
    if let Some(cloid) = child
        .cloid
        .as_deref()
        .map(str::trim)
        .filter(|cloid| !cloid.is_empty())
    {
        format!("slice {cloid}")
    } else if let Some(oid) = child.oid {
        format!("slice #{oid}")
    } else {
        format!("slice {}", child.index)
    }
}
