mod active_symbol;
mod advanced;
mod chase;
mod core;
mod hud;
mod position_actions;
pub(crate) mod pricing;
mod quick_order;
mod sizing;
mod submit;
mod symbols;
mod twap;

pub(crate) use advanced::AdvancedOrderKind;
pub(crate) use core::{
    CancelIntent, ModifyIntent, OneShotPlacementContext, OrderOperation, OrderSurface, PlaceIntent,
    PreparedExchangeOrder, PreparedModifyOrder, PreparedModifyOrderResult, PriceSource,
    QuantityDenomination, QuantitySource, ReduceOnlySource, cancel_order_by_cloid_task,
    cancel_order_task, modify_order_task, place_order_task, validate_surface_market_type,
};
pub(crate) use hud::{HudOrderRequest, HudOrderSide, HudOrderType};
pub(crate) use position_actions::NukePlan;
pub(crate) use sizing::order_size_from_quantity_input;

#[cfg(test)]
pub(crate) use position_actions::{NukePositionOrder, NukeSkipReason};

use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingOrderAction {
    Buy,
    Sell,
    ChaseBuy,
    ChaseSell,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingLeverageUpdateContext {
    pub(crate) address: String,
    pub(crate) symbol_key: String,
    pub(crate) display: String,
    pub(crate) asset: u32,
    pub(crate) dex: Option<String>,
    pub(crate) is_cross: bool,
    pub(crate) leverage: u32,
}

impl PendingLeverageUpdateContext {
    pub(crate) fn margin_mode_label(&self) -> &'static str {
        if self.is_cross { "Cross" } else { "Isolated" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveOrderContextError {
    MissingAgentKey,
    AccountChanged,
}

impl MoveOrderContextError {
    pub(crate) fn status_text(self) -> &'static str {
        match self {
            Self::MissingAgentKey => "Move failed: original agent key is no longer available",
            Self::AccountChanged => {
                "Move stopped: account changed before replacement; original order was cancelled"
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct PendingMoveOrderContext {
    account_address: String,
    agent_key: Zeroizing<String>,
}

impl PendingMoveOrderContext {
    /// Captures the trading identity used to cancel an order so the replacement
    /// cannot silently switch to a different account/key before placement.
    pub(crate) fn new(
        account_address: impl Into<String>,
        agent_key: impl Into<String>,
    ) -> Result<Self, MoveOrderContextError> {
        let agent_key = agent_key.into().trim().to_string();
        if agent_key.is_empty() {
            return Err(MoveOrderContextError::MissingAgentKey);
        }

        Ok(Self {
            account_address: account_address.into(),
            agent_key: agent_key.into(),
        })
    }

    pub(crate) fn replacement_agent_key(
        &self,
        current_account: Option<&str>,
    ) -> Result<Zeroizing<String>, MoveOrderContextError> {
        match current_account {
            Some(current) if current == self.account_address => Ok(self.agent_key.clone()),
            _ => Err(MoveOrderContextError::AccountChanged),
        }
    }
}

/// State for the right-click quick order form on a chart.
pub(crate) struct QuickOrderForm {
    /// Price at the right-click Y coordinate (pre-filled for limit orders).
    pub(crate) price: f64,
    /// User-entered quantity string.
    pub(crate) quantity: String,
    /// True when the quantity field is USD notional, false when it is coin size.
    pub(crate) quantity_is_usd: bool,
    /// Percentage of available notional represented by the current quantity.
    pub(crate) percentage: f32,
    /// True = limit order at clicked price, false = market order.
    pub(crate) is_limit: bool,
    /// Canvas-local X coordinate of the right-click (for card positioning).
    pub(crate) click_x: f32,
    /// Canvas-local Y coordinate of the right-click (for card positioning).
    pub(crate) click_y: f32,
    /// Chart canvas width when clicked.
    pub(crate) chart_w: f32,
    /// Chart canvas height when clicked.
    pub(crate) chart_h: f32,
}
