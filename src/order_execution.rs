mod active_symbol;
mod chase;
mod position_actions;
pub(crate) mod pricing;
mod quick_order;
mod sizing;
mod submit;
mod symbols;
mod twap;

pub(crate) use position_actions::NukePlan;

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
