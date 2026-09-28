use super::{MAX_INFLIGHT_HUD_PLACEMENTS, OrderSurface};
use crate::app_state::TradingTerminal;
use std::fmt;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingOrderAction {
    Buy,
    Sell,
    ChaseBuy,
    ChaseSell,
    ClosePosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingNukeExecution {
    pub(crate) id: u64,
    total: usize,
    completed: usize,
    confirmed: usize,
    failed: usize,
    uncertain: usize,
    skipped: usize,
    refresh_needed: bool,
}

impl PendingNukeExecution {
    pub(crate) fn new(id: u64, total: usize, skipped: usize) -> Self {
        Self {
            id,
            total,
            completed: 0,
            confirmed: 0,
            failed: 0,
            uncertain: 0,
            skipped,
            refresh_needed: false,
        }
    }

    pub(crate) fn record_confirmed(&mut self, refresh_needed: bool) {
        self.completed = self.completed.saturating_add(1);
        self.confirmed = self.confirmed.saturating_add(1);
        self.refresh_needed |= refresh_needed;
    }

    pub(crate) fn record_failed(&mut self, refresh_needed: bool) {
        self.completed = self.completed.saturating_add(1);
        self.failed = self.failed.saturating_add(1);
        self.refresh_needed |= refresh_needed;
    }

    pub(crate) fn record_uncertain(&mut self) {
        self.completed = self.completed.saturating_add(1);
        self.uncertain = self.uncertain.saturating_add(1);
        self.refresh_needed = true;
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.completed >= self.total
    }

    pub(crate) fn refresh_needed(&self) -> bool {
        self.refresh_needed
    }

    pub(crate) fn has_problem(&self) -> bool {
        self.failed > 0 || self.uncertain > 0
    }

    pub(crate) fn status_text(&self) -> String {
        let prefix = if self.is_complete() {
            "NUKE completed"
        } else {
            "NUKE progress"
        };
        let mut status = format!("{prefix}: {}/{} confirmed", self.confirmed, self.total);
        if self.failed > 0 {
            status.push_str(&format!("; {} failed", self.failed));
        }
        if self.uncertain > 0 {
            status.push_str(&format!("; {} uncertain", self.uncertain));
        }
        if self.skipped > 0 {
            status.push_str(&format!("; {} skipped", self.skipped));
        }
        status
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PendingLeverageUpdateContext {
    pub(crate) address: String,
    pub(crate) symbol_key: String,
    pub(crate) display: String,
    pub(crate) asset: u32,
    pub(crate) dex: Option<String>,
    pub(crate) is_cross: bool,
    pub(crate) leverage: u32,
}

impl fmt::Debug for PendingLeverageUpdateContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingLeverageUpdateContext")
            .field("address", &"<redacted>")
            .field("symbol_key", &self.symbol_key)
            .field("display", &self.display)
            .field("asset", &self.asset)
            .field("dex", &self.dex)
            .field("is_cross", &self.is_cross)
            .field("leverage", &self.leverage)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrderLeverageSubmissionSnapshot {
    pub(crate) symbol_key: String,
    pub(crate) leverage_input: String,
    pub(crate) is_cross: bool,
}

impl PendingLeverageUpdateContext {
    pub(crate) fn margin_mode_label(&self) -> &'static str {
        if self.is_cross { "Cross" } else { "Isolated" }
    }
}

impl TradingTerminal {
    pub(crate) fn has_pending_trading_request(&self) -> bool {
        self.pending_order_action.is_some()
            || self.pending_nuke_execution.is_some()
            || self.pending_leverage_update.is_some()
            || !self.pending_one_shot_status_requests.is_empty()
            || self.pending_cancel_status_request.is_some()
            || self.pending_move_status_request.is_some()
            || !self.pending_move_order_contexts.is_empty()
            || self.wallet_clusters.has_pending_execution()
            || self.has_pending_order_indicator_for_connected_account()
            || self.has_inflight_hud_placement_for_connected_account()
    }

    fn has_inflight_hud_placement_for_connected_account(&self) -> bool {
        self.connected_order_account_address()
            .is_some_and(|address| self.hud_placements.has_any_for_account(&address))
    }

    /// HUD limit clicks are allowed to overlap each other, so this variant of
    /// [`Self::has_pending_trading_request`] ignores HUD-placement-originated
    /// state (the in-flight tracker, its chart indicators, and HUD one-shot
    /// status checks) while still blocking on every other surface.
    fn has_pending_trading_request_blocking_hud_placement(&self) -> bool {
        self.pending_order_action.is_some()
            || self.pending_nuke_execution.is_some()
            || self.pending_leverage_update.is_some()
            || self
                .pending_one_shot_status_requests
                .values()
                .any(|pending| pending.surface() != OrderSurface::Hud)
            || self.pending_cancel_status_request.is_some()
            || self.pending_move_status_request.is_some()
            || !self.pending_move_order_contexts.is_empty()
            || self.wallet_clusters.has_pending_execution()
            || self.has_non_hud_pending_order_indicator_for_connected_account()
    }

    pub(crate) fn reject_if_pending_trading_request(&mut self, action: &str) -> bool {
        if !self.has_pending_trading_request() {
            return false;
        }

        self.order_status = Some((
            format!("Wait for pending trading requests to finish before {action}"),
            true,
        ));
        true
    }

    pub(crate) fn reject_if_pending_trading_request_blocking_hud_placement(
        &mut self,
        action: &str,
    ) -> bool {
        if !self.has_pending_trading_request_blocking_hud_placement() {
            return false;
        }

        self.order_status = Some((
            format!("Wait for pending trading requests to finish before {action}"),
            true,
        ));
        true
    }

    pub(crate) fn reject_if_hud_placement_limit_reached(&mut self) -> bool {
        let inflight = self
            .connected_order_account_address()
            .map(|address| self.hud_placements.count_for_account(&address))
            .unwrap_or(0);
        if inflight < MAX_INFLIGHT_HUD_PLACEMENTS {
            return false;
        }

        self.order_status = Some((
            "Too many HUD orders in flight; wait for confirmations".to_string(),
            true,
        ));
        true
    }
}
