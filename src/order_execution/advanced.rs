use crate::app_state::TradingTerminal;
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Advanced Order Startup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdvancedOrderKind {
    Chase,
    Twap,
}

impl AdvancedOrderKind {
    fn start_label(self) -> &'static str {
        match self {
            Self::Chase => "chase",
            Self::Twap => "TWAP",
        }
    }
}

pub(super) struct AdvancedOrderStartContext {
    pub(super) account_address: String,
    pub(super) agent_key: Zeroizing<String>,
}

impl TradingTerminal {
    pub(crate) fn active_advanced_order_count(&self) -> usize {
        self.chase_orders
            .values()
            .filter(|chase| !chase.lifecycle.is_stopping())
            .count()
            + self
                .twap_orders
                .values()
                .filter(|twap| !twap.status.is_terminal() && !twap.stop_requested)
                .count()
    }

    pub(super) fn advanced_order_start_context(
        &mut self,
        kind: AdvancedOrderKind,
    ) -> Option<AdvancedOrderStartContext> {
        if self.active_advanced_order_count() >= MAX_ACTIVE_ADVANCED_ORDERS {
            self.order_status = Some((
                format!(
                    concat!(
                        "Cannot start {}: maximum of {} ",
                        "active advanced orders reached"
                    ),
                    kind.start_label(),
                    MAX_ACTIVE_ADVANCED_ORDERS
                ),
                true,
            ));
            return None;
        }

        if self.pending_order_action.is_some() {
            self.order_status = Some(("Wait for the pending order action to finish".into(), true));
            return None;
        }

        let agent_key = self.wallet_key_input.trim().to_string();
        let Some(account_address) = self.connected_address.clone() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        };
        if agent_key.is_empty() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        }

        if self.symbol_key_is_hidden(&self.active_symbol) {
            self.order_status = Some(("Active ticker is hidden in Settings > Risk".into(), true));
            return None;
        }

        Some(AdvancedOrderStartContext {
            account_address,
            agent_key: agent_key.into(),
        })
    }
}

#[cfg(test)]
mod tests;
