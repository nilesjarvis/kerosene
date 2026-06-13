use crate::app_state::TradingTerminal;
use crate::config::MarketUniverseConfig;
use crate::signing::CapturedAgentKey;
use crate::signing::OrderKind;
use crate::twap_state::{MAX_ACTIVE_ADVANCED_ORDERS, TwapOrderForm};

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

    fn title_label(self) -> &'static str {
        match self {
            Self::Chase => "Chase",
            Self::Twap => "TWAP",
        }
    }
}

pub(super) struct AdvancedOrderStartContext {
    pub(super) account_address: String,
    pub(super) agent_key: CapturedAgentKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdvancedOrderStartSnapshot {
    pub(crate) order_kind: OrderKind,
    pub(crate) symbol_key: String,
    pub(crate) quantity_input: String,
    pub(crate) quantity_is_usd: bool,
    pub(crate) reduce_only: bool,
    pub(crate) market_universe: MarketUniverseConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TwapOrderStartSnapshot {
    pub(crate) order: AdvancedOrderStartSnapshot,
    pub(crate) twap_form: TwapOrderForm,
}

impl TradingTerminal {
    pub(crate) fn advanced_order_start_snapshot(&self) -> AdvancedOrderStartSnapshot {
        AdvancedOrderStartSnapshot {
            order_kind: self.order_kind,
            symbol_key: self.active_symbol.clone(),
            quantity_input: self.order_quantity.clone(),
            quantity_is_usd: self.order_quantity_is_usd,
            reduce_only: self.order_reduce_only,
            market_universe: self.market_universe.clone(),
        }
    }

    pub(crate) fn twap_order_start_snapshot(&self) -> TwapOrderStartSnapshot {
        TwapOrderStartSnapshot {
            order: self.advanced_order_start_snapshot(),
            twap_form: self.twap_form.clone(),
        }
    }

    pub(crate) fn advanced_order_start_snapshot_matches(
        &self,
        snapshot: &AdvancedOrderStartSnapshot,
    ) -> bool {
        self.order_kind == snapshot.order_kind
            && self.active_symbol == snapshot.symbol_key
            && self.order_quantity == snapshot.quantity_input
            && self.order_quantity_is_usd == snapshot.quantity_is_usd
            && self.order_reduce_only == snapshot.reduce_only
            && self.market_universe == snapshot.market_universe
    }

    pub(crate) fn twap_order_start_snapshot_matches(
        &self,
        snapshot: &TwapOrderStartSnapshot,
    ) -> bool {
        self.advanced_order_start_snapshot_matches(&snapshot.order)
            && self.twap_form == snapshot.twap_form
    }

    pub(crate) fn reject_stale_advanced_order_start_snapshot(
        &mut self,
        kind: AdvancedOrderKind,
        snapshot: &AdvancedOrderStartSnapshot,
    ) -> bool {
        if self.advanced_order_start_snapshot_matches(snapshot) {
            return false;
        }

        self.order_status = Some((
            format!(
                "{} settings changed; review and start again",
                kind.title_label()
            ),
            true,
        ));
        true
    }

    pub(crate) fn reject_stale_twap_order_start_snapshot(
        &mut self,
        snapshot: &TwapOrderStartSnapshot,
    ) -> bool {
        if self.twap_order_start_snapshot_matches(snapshot) {
            return false;
        }

        self.order_status = Some(("TWAP settings changed; review and start again".into(), true));
        true
    }

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

    fn reject_if_advanced_order_start_blocked(&mut self, kind: AdvancedOrderKind) -> bool {
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
            return true;
        }

        if self.reject_if_pending_trading_request(&format!("starting a {}", kind.start_label())) {
            return true;
        }
        self.reject_if_account_reconciliation_required(
            &format!("starting a {}", kind.start_label()),
            "account data",
        )
    }

    pub(super) fn advanced_order_start_context(
        &mut self,
        kind: AdvancedOrderKind,
    ) -> Option<AdvancedOrderStartContext> {
        if self.reject_if_advanced_order_start_blocked(kind) {
            return None;
        }

        let (agent_key, account_address) = self.captured_order_signing_context()?;

        if self.symbol_key_is_hidden(&self.active_symbol) {
            self.order_status = Some(("Active ticker is hidden in Settings > Risk".into(), true));
            return None;
        }

        Some(AdvancedOrderStartContext {
            account_address,
            agent_key,
        })
    }

    pub(crate) fn advanced_order_start_preflight_ready(&mut self, kind: AdvancedOrderKind) -> bool {
        if self.reject_if_advanced_order_start_blocked(kind) {
            return false;
        }
        if self.checked_order_signing_account().is_none() {
            return false;
        }
        if self.symbol_key_is_hidden(&self.active_symbol) {
            self.order_status = Some(("Active ticker is hidden in Settings > Risk".into(), true));
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests;
