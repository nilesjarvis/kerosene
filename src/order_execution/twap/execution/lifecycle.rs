use crate::app_state::TradingTerminal;
use crate::twap_state::{
    ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL, TwapChildStatus, TwapEventKind, TwapStatus,
};

use std::time::Instant;

// ---------------------------------------------------------------------------
// TWAP Slice Lifecycle
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn record_twap_skip(
        &mut self,
        twap_id: u64,
        now: Instant,
        kind: TwapEventKind,
        message: String,
        is_error: bool,
    ) {
        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            twap.slices_attempted = twap.slices_attempted.saturating_add(1);
            twap.push_event(kind, message.clone(), is_error);
            self.order_status = Some((message, is_error));
            twap.schedule_after_attempt(now);
        }
    }

    pub(super) fn record_twap_retry_skip(
        &mut self,
        twap_id: u64,
        now: Instant,
        slice_index: u32,
        kind: TwapEventKind,
        message: String,
        is_error: bool,
    ) {
        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            twap.retry_slice = None;
            if let Some(child) = twap.child_order_mut(slice_index) {
                child.status = TwapChildStatus::NoFill;
                child.exchange_summary = message.clone();
            }
            twap.push_event(kind, message.clone(), is_error);
            self.order_status = Some((message, is_error));
            twap.schedule_after_attempt(now);
        }
    }

    pub(in crate::order_execution::twap) fn finish_twap_attempt(
        &mut self,
        twap_id: u64,
        now: Instant,
    ) {
        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            if twap.status.is_terminal() {
                return;
            }
            if twap.stop_requested {
                let (message, is_error) = twap
                    .stop_reason
                    .clone()
                    .unwrap_or_else(|| ("TWAP stopped".to_string(), false));
                twap.status = TwapStatus::Stopped;
                twap.push_event(TwapEventKind::Stopped, message.clone(), is_error);
                self.order_status = Some((message, is_error));
            } else if twap.pending_op.is_none() && !twap.status.is_terminal() {
                twap.schedule_after_attempt(now);
            }
        }
        self.archive_twap_if_terminal(twap_id);
    }

    pub(in crate::order_execution::twap) fn expire_twap_if_deadline_passed(
        &mut self,
        twap_id: u64,
        now: Instant,
    ) -> bool {
        let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
            return true;
        };
        if now < twap.ends_at
            || twap.pending_op.is_some()
            || twap.status_check_cloid.is_some()
            || twap.has_status_unknown_child()
            || twap.status.is_terminal()
        {
            return false;
        }
        twap.slices_attempted = twap.slice_count;
        twap.status = if twap.filled_size > 0.0 {
            TwapStatus::CompletedPartial
        } else {
            TwapStatus::Stopped
        };
        let message = if twap.filled_size > 0.0 {
            "TWAP ended at deadline with unfilled remainder".to_string()
        } else {
            "TWAP ended at deadline without fills".to_string()
        };
        twap.push_event(TwapEventKind::Completed, message.clone(), false);
        self.order_status = Some((message, false));
        self.archive_twap_if_terminal(twap_id);
        true
    }

    pub(in crate::order_execution::twap) fn can_send_advanced_exchange_request(
        &self,
        now: Instant,
    ) -> bool {
        !self.account_loading
            && !self.account_reconciliation_required
            && self.last_advanced_exchange_request_at.is_none_or(|last| {
                now.saturating_duration_since(last) >= ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL
            })
    }
}
