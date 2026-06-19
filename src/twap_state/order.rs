use self::schedule::twap_seed;
use super::TWAP_EVENT_LIMIT;
use super::model::{
    TwapChildOrder, TwapEvent, TwapEventKind, TwapOrder, TwapOrderInit, TwapPauseReason, TwapStatus,
};
use crate::helpers::positive_finite_value;

use std::time::Instant;

mod reconciliation;
mod schedule;

// ---------------------------------------------------------------------------
// TWAP Order Behavior
// ---------------------------------------------------------------------------

impl TwapOrder {
    pub(crate) fn new(init: TwapOrderInit) -> Self {
        let TwapOrderInit {
            id,
            coin,
            display_coin,
            account_address,
            agent_key,
            is_buy,
            target_size,
            asset,
            sz_decimals,
            is_spot,
            reduce_only,
            min_price,
            max_price,
            randomize,
            duration,
            slice_count,
            now,
            started_at_ms,
        } = init;
        let mut order = Self {
            id,
            coin,
            display_coin,
            account_address,
            agent_key,
            is_buy,
            target_size,
            remaining_size: target_size,
            filled_size: 0.0,
            asset,
            sz_decimals,
            is_spot,
            reduce_only,
            min_price,
            max_price,
            randomize,
            random_seed: twap_seed(id, now),
            duration,
            slice_count,
            slices_attempted: 0,
            slices_sent: 0,
            started_at: now,
            started_at_ms,
            ends_at: now + duration,
            next_slice_due: now,
            pending_op: None,
            latest_book: None,
            status: TwapStatus::WaitingForMarket,
            pause_reason: None,
            paused_until: None,
            retry_slice: None,
            status_check_cloid: None,
            status_check_retries: 0,
            account_reconciliation_retries: 0,
            reconciliation_deadline: None,
            cancel_retries: 0,
            stop_requested: false,
            stop_reason: None,
            child_orders: Vec::new(),
            events: Vec::new(),
            window_id: None,
        };
        order.push_event(TwapEventKind::Started, "TWAP started".to_string(), false);
        order
    }

    pub(crate) fn side_label(&self) -> &'static str {
        if self.is_buy { "BUY" } else { "SELL" }
    }

    pub(crate) fn child_order_mut(&mut self, index: u32) -> Option<&mut TwapChildOrder> {
        self.child_orders
            .iter_mut()
            .find(|child| child.index == index)
    }

    pub(crate) fn update_child_orders_matching(
        &mut self,
        mut matches: impl FnMut(&TwapChildOrder) -> bool,
        mut update: impl FnMut(&mut TwapChildOrder),
    ) {
        for child in &mut self.child_orders {
            if matches(child) {
                update(child);
            }
        }
    }

    pub(crate) fn pause(
        &mut self,
        reason: TwapPauseReason,
        paused_until: Option<Instant>,
        message: String,
        is_error: bool,
    ) {
        if self.status.is_terminal() {
            return;
        }
        self.status = if self.stop_requested {
            TwapStatus::Stopping
        } else {
            TwapStatus::Paused
        };
        self.pause_reason = Some(reason);
        self.paused_until = paused_until;
        if let Some(until) = paused_until {
            self.next_slice_due = until;
        }
        self.push_event(TwapEventKind::Paused, message, is_error);
    }

    pub(crate) fn clear_pause(&mut self) {
        self.pause_reason = None;
        self.paused_until = None;
        self.status_check_retries = 0;
        if !self.status.is_terminal() && !self.stop_requested && self.pending_op.is_none() {
            self.status = TwapStatus::WaitingForMarket;
        }
    }

    pub(crate) fn progress_fraction(&self) -> f64 {
        if positive_finite_value(self.target_size).is_some() {
            (self.filled_size / self.target_size).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub(crate) fn push_event(&mut self, kind: TwapEventKind, message: String, is_error: bool) {
        self.events.push(TwapEvent {
            at: Instant::now(),
            kind,
            message,
            is_error,
        });
        if self.events.len() > TWAP_EVENT_LIMIT {
            let excess = self.events.len().saturating_sub(TWAP_EVENT_LIMIT);
            self.events.drain(0..excess);
        }
    }
}

impl std::fmt::Debug for TwapOrder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let has_pending_op = self.pending_op.is_some();
        let has_latest_book = self.latest_book.is_some();
        let has_retry_slice = self.retry_slice.is_some();
        let has_status_check_cloid = self.status_check_cloid.is_some();
        let stop_reason_is_error = self.stop_reason.as_ref().map(|(_, is_error)| *is_error);

        formatter
            .debug_struct("TwapOrder")
            .field("id", &self.id)
            .field("coin", &"<redacted>")
            .field("display_coin", &"<redacted>")
            .field("account_address", &"<redacted>")
            .field("agent_key", &"<redacted>")
            .field("is_buy", &self.is_buy)
            .field("target_size", &"<redacted>")
            .field("remaining_size", &"<redacted>")
            .field("filled_size", &"<redacted>")
            .field("asset", &self.asset)
            .field("sz_decimals", &self.sz_decimals)
            .field("is_spot", &self.is_spot)
            .field("reduce_only", &self.reduce_only)
            .field("min_price", &"<redacted>")
            .field("max_price", &"<redacted>")
            .field("randomize", &self.randomize)
            .field("duration", &"<redacted>")
            .field("slice_count", &"<redacted>")
            .field("slices_attempted", &self.slices_attempted)
            .field("slices_sent", &self.slices_sent)
            .field("has_pending_op", &has_pending_op)
            .field("has_latest_book", &has_latest_book)
            .field("status", &self.status)
            .field("pause_reason", &self.pause_reason)
            .field("paused_until", &self.paused_until)
            .field("has_retry_slice", &has_retry_slice)
            .field("has_status_check_cloid", &has_status_check_cloid)
            .field("status_check_retries", &self.status_check_retries)
            .field(
                "account_reconciliation_retries",
                &self.account_reconciliation_retries,
            )
            .field("reconciliation_deadline", &self.reconciliation_deadline)
            .field("cancel_retries", &self.cancel_retries)
            .field("stop_requested", &self.stop_requested)
            .field("stop_reason_is_error", &stop_reason_is_error)
            .field("child_orders_count", &self.child_orders.len())
            .field("events_count", &self.events.len())
            .finish()
    }
}
