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

pub(crate) use advanced::{AdvancedOrderKind, AdvancedOrderStartSnapshot, TwapOrderStartSnapshot};
pub(crate) use core::{
    CancelIntent, MarketUsdSizeReference, ModifyIntent, OneShotPlacementContext, OrderOperation,
    OrderSurface, PlaceIntent, PreparedExchangeOrder, PreparedModifyOrder,
    PreparedModifyOrderResult, PriceSource, QuantityDenomination, QuantitySource, ReduceOnlySource,
    cancel_order_by_cloid_task, cancel_order_task, modify_order_task, place_order_task,
    validate_surface_market_type,
};
pub(crate) use hud::{
    HudOrderRequest, HudOrderSide, HudOrderType, HudPlacementTracker, MAX_INFLIGHT_HUD_PLACEMENTS,
};
pub(crate) use position_actions::{NukePlan, reject_if_positions_incomplete_for_action};
pub(crate) use quick_order::QuickOrderSubmissionSnapshot;
pub(crate) use sizing::order_size_from_quantity_input;
pub(crate) use submit::{TicketOrderPlaceIntent, TicketOrderSubmissionSnapshot};

#[cfg(test)]
pub(crate) use position_actions::{NukePositionOrder, NukeSkipReason};

use crate::account::{AccountData, OpenOrder};
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::config;
use crate::signing::{CapturedAgentKey, ChaseOrder};
use std::{collections::BTreeSet, fmt};
use zeroize::Zeroizing;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SpotAutomationSymbolIdentity {
    key: String,
    ticker: String,
    display_name: Option<String>,
    asset_index: u32,
    quote_token: Option<u32>,
    sz_decimals: u32,
}

impl SpotAutomationSymbolIdentity {
    fn from_symbol(symbol: &crate::api::ExchangeSymbol) -> Option<Self> {
        (symbol.market_type == MarketType::Spot).then(|| Self {
            key: symbol.key.clone(),
            ticker: symbol.ticker.clone(),
            display_name: symbol.display_name.clone(),
            asset_index: symbol.asset_index,
            quote_token: symbol.collateral_token,
            sz_decimals: symbol.sz_decimals,
        })
    }

    fn matches(&self, symbol: &crate::api::ExchangeSymbol) -> bool {
        symbol.market_type == MarketType::Spot
            && self.key == symbol.key
            && self.ticker == symbol.ticker
            && self.display_name == symbol.display_name
            && self.asset_index == symbol.asset_index
            && self.quote_token == symbol.collateral_token
            && self.sz_decimals == symbol.sz_decimals
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingOrderAction {
    Buy,
    Sell,
    ChaseBuy,
    ChaseSell,
    ClosePosition,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PendingNukeExecution {
    pub(crate) id: u64,
    total: usize,
    settled_child_cloids: BTreeSet<String>,
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
            settled_child_cloids: BTreeSet::new(),
            confirmed: 0,
            failed: 0,
            uncertain: 0,
            skipped,
            refresh_needed: false,
        }
    }

    fn record_child_settled(&mut self, child_cloid: &str) -> bool {
        self.settled_child_cloids.insert(child_cloid.to_string())
    }

    pub(crate) fn record_confirmed(&mut self, child_cloid: &str, refresh_needed: bool) -> bool {
        if !self.record_child_settled(child_cloid) {
            return false;
        }
        self.confirmed = self.confirmed.saturating_add(1);
        self.refresh_needed |= refresh_needed;
        true
    }

    pub(crate) fn record_failed(&mut self, child_cloid: &str, refresh_needed: bool) -> bool {
        if !self.record_child_settled(child_cloid) {
            return false;
        }
        self.failed = self.failed.saturating_add(1);
        self.refresh_needed |= refresh_needed;
        true
    }

    pub(crate) fn record_uncertain(&mut self, child_cloid: &str) -> bool {
        if !self.record_child_settled(child_cloid) {
            return false;
        }
        self.uncertain = self.uncertain.saturating_add(1);
        self.refresh_needed = true;
        true
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.settled_child_cloids.len() >= self.total
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

impl fmt::Debug for PendingNukeExecution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingNukeExecution")
            .field("id", &self.id)
            .field("total", &self.total)
            .field("completed", &self.settled_child_cloids.len())
            .field("confirmed", &self.confirmed)
            .field("failed", &self.failed)
            .field("uncertain", &self.uncertain)
            .field("skipped", &self.skipped)
            .field("refresh_needed", &self.refresh_needed)
            .finish()
    }
}

pub(crate) fn order_account_addresses_match(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();
    !left.is_empty() && !right.is_empty() && left.eq_ignore_ascii_case(right)
}

fn chase_open_order_side_is_buy(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

pub(crate) fn open_order_matches_chase_identity(chase: &ChaseOrder, order: &OpenOrder) -> bool {
    chase.tracks_oid(order.oid)
        && order.coin == chase.coin
        && chase_open_order_side_is_buy(&order.side) == Some(chase.is_buy)
        && (chase.is_spot || order.reduce_only == Some(chase.reduce_only))
}

impl TradingTerminal {
    pub(crate) fn record_chase_spot_symbol_identity(
        &mut self,
        chase_id: u64,
        symbol: &crate::api::ExchangeSymbol,
    ) {
        if let Some(identity) = SpotAutomationSymbolIdentity::from_symbol(symbol) {
            self.chase_spot_symbol_identities.insert(chase_id, identity);
        }
    }

    pub(crate) fn record_twap_spot_symbol_identity(
        &mut self,
        twap_id: u64,
        symbol: &crate::api::ExchangeSymbol,
    ) {
        if let Some(identity) = SpotAutomationSymbolIdentity::from_symbol(symbol) {
            self.twap_spot_symbol_identities.insert(twap_id, identity);
        }
    }

    pub(crate) fn chase_spot_symbol_identity_is_current(
        &self,
        chase_id: u64,
        symbol_key: &str,
    ) -> bool {
        self.chase_spot_symbol_identities
            .get(&chase_id)
            .zip(self.exchange_symbol_for_key(symbol_key))
            .is_some_and(|(identity, symbol)| identity.matches(symbol))
    }

    pub(crate) fn twap_spot_symbol_identity_is_current(
        &self,
        twap_id: u64,
        symbol_key: &str,
    ) -> bool {
        self.twap_spot_symbol_identities
            .get(&twap_id)
            .zip(self.exchange_symbol_for_key(symbol_key))
            .is_some_and(|(identity, symbol)| identity.matches(symbol))
    }

    pub(crate) fn connected_order_account_address(&self) -> Option<String> {
        self.connected_address
            .as_deref()
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .map(str::to_string)
    }

    pub(crate) fn connected_order_account_matches(&self, account_address: &str) -> bool {
        self.connected_address
            .as_deref()
            .is_some_and(|connected| order_account_addresses_match(connected, account_address))
    }

    pub(crate) fn account_data_for_order_account(
        &self,
        account_address: &str,
    ) -> Option<&AccountData> {
        let account_address = account_address.trim();
        if account_address.is_empty() {
            return None;
        }
        self.account_data.as_ref().filter(|_| {
            self.account_data_address
                .as_deref()
                .is_some_and(|owner| order_account_addresses_match(owner, account_address))
        })
    }

    pub(crate) fn account_data_for_order_account_mut(
        &mut self,
        account_address: &str,
    ) -> Option<&mut AccountData> {
        let account_address = account_address.trim();
        if account_address.is_empty() {
            return None;
        }
        let owner_matches = self
            .account_data_address
            .as_deref()
            .is_some_and(|owner| order_account_addresses_match(owner, account_address));
        if owner_matches {
            self.account_data.as_mut()
        } else {
            None
        }
    }

    /// A spot placement, modification, or cancellation can change exchange
    /// holds before the next `spotState` frame arrives. Do not let a balance
    /// snapshot captured before dispatch drive another percentage-sized order.
    pub(crate) fn invalidate_spot_balances_after_exchange_dispatch(
        &mut self,
        account_address: &str,
        market_type: MarketType,
    ) {
        if market_type != MarketType::Spot || !self.connected_order_account_matches(account_address)
        {
            return;
        }

        let invalidated = self
            .account_data_for_order_account_mut(account_address)
            .map(|data| {
                data.completeness.spot_balances_complete = false;
            })
            .is_some();
        if invalidated {
            self.bump_spot_balances_revision();
        }
    }

    pub(crate) fn connected_order_account_snapshot(&self) -> Option<(String, &AccountData)> {
        let account_address = self.connected_order_account_address()?;
        let data = self.account_data_for_order_account(&account_address)?;
        Some((account_address, data))
    }

    pub(crate) fn reject_if_account_reconciliation_required(
        &mut self,
        action: &str,
        data_label: &str,
    ) -> bool {
        if !self.account_reconciliation_required {
            return false;
        }

        self.order_status = Some((
            format!("Account refresh pending; wait for fresh {data_label} before {action}"),
            true,
        ));
        true
    }

    pub(crate) fn clear_pending_move_order_state(&mut self) {
        self.pending_move_order_contexts.clear();
        self.active_move_order_drag = None;
    }

    pub(crate) fn active_wallet_context_matches_connected_account(
        &self,
        account_address: &str,
    ) -> bool {
        let Some(connected) = config::SecretPayload::normalize_wallet_address(account_address)
        else {
            return false;
        };
        let input_matches =
            config::SecretPayload::normalize_wallet_address(&self.wallet_address_input)
                .is_some_and(|address| address == connected);
        let active_profile_matches = self
            .accounts
            .get(self.active_account_index)
            .and_then(|profile| {
                config::SecretPayload::normalize_wallet_address(&profile.wallet_address)
            })
            .is_some_and(|address| address == connected);

        input_matches && active_profile_matches
    }

    fn reject_mismatched_trading_context(&mut self, account_address: &str) -> bool {
        if self.active_wallet_context_matches_connected_account(account_address) {
            return false;
        }

        self.order_status = Some((
            "Connected wallet no longer matches the active account; reconnect before trading"
                .into(),
            true,
        ));
        true
    }

    pub(crate) fn active_committed_agent_key(&self) -> Zeroizing<String> {
        Zeroizing::new(
            self.accounts
                .get(self.active_account_index)
                .map(|profile| profile.agent_key.trim().to_string())
                .unwrap_or_default(),
        )
    }

    pub(crate) fn has_active_committed_agent_key(&self) -> bool {
        self.active_committed_agent_key_is_present()
    }

    fn active_committed_agent_key_is_present(&self) -> bool {
        self.accounts
            .get(self.active_account_index)
            .is_some_and(|profile| !profile.agent_key.trim().is_empty())
    }

    pub(crate) fn checked_order_signing_account(&mut self) -> Option<String> {
        if !self.active_committed_agent_key_is_present() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        }
        let Some(account_address) = self.connected_order_account_address() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        };
        if self.reject_mismatched_trading_context(&account_address) {
            return None;
        }

        Some(account_address)
    }

    pub(crate) fn order_signing_context(&mut self) -> Option<(Zeroizing<String>, String)> {
        let key = self.active_committed_agent_key();
        if key.is_empty() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        }
        let Some(account_address) = self.connected_order_account_address() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        };
        if self.reject_mismatched_trading_context(&account_address) {
            return None;
        }

        Some((key, account_address))
    }

    pub(crate) fn captured_order_signing_context(&mut self) -> Option<(CapturedAgentKey, String)> {
        let key = CapturedAgentKey::new(self.active_committed_agent_key());
        let Some(account_address) = self.connected_order_account_address() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        };
        let Some(key) = key else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        };
        if self.reject_mismatched_trading_context(&account_address) {
            return None;
        }

        Some((key, account_address))
    }

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
            .field("symbol_key", &"<redacted>")
            .field("display", &"<redacted>")
            .field("asset", &"<redacted>")
            .field("dex", &self.dex.as_ref().map(|_| "<redacted>"))
            .field("is_cross", &self.is_cross)
            .field("leverage", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OrderLeverageSubmissionSnapshot {
    pub(crate) symbol_key: String,
    pub(crate) leverage_input: String,
    pub(crate) is_cross: bool,
}

impl fmt::Debug for OrderLeverageSubmissionSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderLeverageSubmissionSnapshot")
            .field("symbol_key", &"<redacted>")
            .field("leverage_input", &"<redacted>")
            .field("is_cross", &self.is_cross)
            .finish()
    }
}

impl PendingLeverageUpdateContext {
    pub(crate) fn margin_mode_label(&self) -> &'static str {
        if self.is_cross { "Cross" } else { "Isolated" }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MoveOrderKey, OneShotPlacementContext, OrderSurface, PendingLeverageUpdateContext,
        PendingMoveOrderContext, PendingNukeExecution, PendingOrderAction, QuickOrderForm,
        QuickOrderQuantityProvenance, QuickOrderRecovery,
    };
    use crate::account::{
        AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
        SpotClearinghouseState,
    };
    use crate::api::MarketType;
    use crate::app_state::{TradingTerminal, sensitive_string};
    use crate::chart_state::ChartSurfaceId;
    use crate::config::AccountProfile;
    use crate::order_update::{
        PendingCancelStatusRequest, PendingMoveStatusRequest, PendingOneShotStatusRequest,
    };
    use crate::signing::ExchangeOrderKind;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
    const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

    fn connect_test_account(terminal: &mut TradingTerminal) {
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Account A".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: sensitive_string("").into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }];
        terminal.active_account_index = 0;
    }

    fn empty_account_data() -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: Default::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: 1,
        }
    }

    #[test]
    fn quick_order_quantity_provenance_debug_redacts_account_address() {
        let provenance = QuickOrderQuantityProvenance {
            account_address: TEST_ACCOUNT.to_string(),
            account_data_revision: 7,
            spot_balances_revision: 3,
            symbol_key: "SECRETCOIN".to_string(),
            quantity_is_usd: true,
            percentage: 42.42,
            is_limit: false,
            reference_price: Some(98765.4321),
            reduce_only: false,
            market_universe: crate::config::MarketUniverseConfig::default(),
        };

        let rendered = format!("{provenance:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(TEST_ACCOUNT));
        assert!(!rendered.contains("SECRETCOIN"));
        assert!(!rendered.contains("42.42"));
        assert!(!rendered.contains("98765.4321"));
    }

    #[test]
    fn quick_order_form_and_recovery_debug_redact_order_details() {
        let form = QuickOrderForm {
            price: 98765.4321,
            quantity: "quantity-secret".to_string(),
            quantity_is_usd: true,
            percentage: 42.42,
            quantity_provenance: Some(QuickOrderQuantityProvenance {
                account_address: TEST_ACCOUNT.to_string(),
                account_data_revision: 7,
                spot_balances_revision: 3,
                symbol_key: "SECRETCOIN".to_string(),
                quantity_is_usd: true,
                percentage: 42.42,
                is_limit: true,
                reference_price: Some(12345.6789),
                reduce_only: false,
                market_universe: crate::config::MarketUniverseConfig::default(),
            }),
            is_limit: true,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 400.0,
            chart_h: 300.0,
        };
        let recovery = QuickOrderRecovery {
            chart_id: 1,
            form,
            surface_id: Some(ChartSurfaceId::Docked(1)),
        };

        let rendered = format!("{recovery:?}");

        assert!(rendered.contains("price: <redacted>"));
        assert!(rendered.contains("quantity: <redacted>"));
        assert!(rendered.contains("quantity_provenance: Some(\"<redacted>\")"));
        assert!(rendered.contains("chart_id: 1"));
        for secret in [
            TEST_ACCOUNT,
            "SECRETCOIN",
            "quantity-secret",
            "98765.4321",
            "12345.6789",
            "42.42",
        ] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    #[test]
    fn connected_order_account_address_rejects_missing_and_blank_values() {
        let mut terminal = TradingTerminal::boot().0;

        terminal.connected_address = None;
        assert_eq!(terminal.connected_order_account_address(), None);

        terminal.connected_address = Some(String::new());
        assert_eq!(terminal.connected_order_account_address(), None);

        terminal.connected_address = Some("   ".to_string());
        assert_eq!(terminal.connected_order_account_address(), None);
        assert!(!terminal.connected_order_account_matches("   "));
    }

    #[test]
    fn connected_order_account_address_trims_surrounding_whitespace() {
        let mut terminal = TradingTerminal::boot().0;

        terminal.connected_address = Some(" 0xabc ".to_string());

        assert_eq!(
            terminal.connected_order_account_address(),
            Some("0xabc".to_string())
        );
        assert!(terminal.connected_order_account_matches("0xabc"));
        assert!(terminal.connected_order_account_matches(" 0xabc "));
        assert!(terminal.connected_order_account_matches(" 0XABC "));
    }

    #[test]
    fn account_data_for_order_account_normalizes_case_and_whitespace() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.account_data_address = Some(" 0xAbC ".to_string());
        terminal.account_data = Some(empty_account_data());

        assert!(terminal.account_data_for_order_account(" 0xabc ").is_some());
        assert!(
            terminal
                .account_data_for_order_account_mut(" 0XABC ")
                .is_some()
        );
        assert!(terminal.account_data_for_order_account("0xdef").is_none());
    }

    #[test]
    fn spot_exchange_dispatch_invalidates_only_the_connected_accounts_spot_balances() {
        let mut terminal = TradingTerminal::boot().0;
        connect_test_account(&mut terminal);
        terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
        let mut data = empty_account_data();
        data.completeness.spot_balances_complete = true;
        data.completeness.spot_balances_fetched_at_ms = Some(123);
        terminal.account_data = Some(data);
        let initial_revision = terminal.spot_balances_revision;

        terminal.invalidate_spot_balances_after_exchange_dispatch(TEST_ACCOUNT, MarketType::Spot);

        assert!(
            !terminal
                .account_data
                .as_ref()
                .expect("account data")
                .completeness
                .spot_balances_complete
        );
        assert_eq!(
            terminal.spot_balances_revision,
            initial_revision.wrapping_add(1)
        );

        terminal
            .account_data
            .as_mut()
            .expect("account data")
            .completeness
            .spot_balances_complete = true;
        let revision_after_spot = terminal.spot_balances_revision;
        terminal.invalidate_spot_balances_after_exchange_dispatch(TEST_ACCOUNT, MarketType::Perp);
        terminal.invalidate_spot_balances_after_exchange_dispatch(OTHER_ACCOUNT, MarketType::Spot);

        assert!(
            terminal
                .account_data
                .as_ref()
                .expect("account data")
                .completeness
                .spot_balances_complete
        );
        assert_eq!(terminal.spot_balances_revision, revision_after_spot);
    }

    #[test]
    fn pending_trading_request_tracks_all_account_transition_blockers() {
        let account = TEST_ACCOUNT;
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(account.to_string());
        assert!(!terminal.has_pending_trading_request());

        terminal.pending_order_action = Some(PendingOrderAction::Buy);
        assert!(terminal.has_pending_trading_request());
        terminal.pending_order_action = None;

        terminal.pending_nuke_execution = Some(PendingNukeExecution::new(1, 1, 0));
        assert!(terminal.has_pending_trading_request());
        terminal.pending_nuke_execution = None;

        terminal.pending_leverage_update = Some(PendingLeverageUpdateContext {
            address: account.to_string(),
            symbol_key: "BTC".to_string(),
            display: "BTC".to_string(),
            asset: 0,
            dex: None,
            is_cross: true,
            leverage: 3,
        });
        assert!(terminal.has_pending_trading_request());
        terminal.pending_leverage_update = None;

        terminal.insert_pending_one_shot_status_request(PendingOneShotStatusRequest::new(
            7,
            &OneShotPlacementContext {
                account_address: account.to_string(),
                cloid: "0x00000000000000000000000000000000".to_string(),
                surface: OrderSurface::Ticket,
                symbol_key: "BTC".to_string(),
                order_kind: ExchangeOrderKind::Limit,
            },
        ));
        assert!(terminal.has_pending_trading_request());
        terminal.pending_one_shot_status_requests.clear();

        terminal.pending_cancel_status_request = Some(PendingCancelStatusRequest::new(
            7,
            account.to_string(),
            42,
            "BTC".to_string(),
        ));
        assert!(terminal.has_pending_trading_request());
        terminal.pending_cancel_status_request = None;

        terminal.pending_move_status_request = Some(PendingMoveStatusRequest::new(
            8,
            account.to_string(),
            42,
            "BTC".to_string(),
            "100".to_string(),
        ));
        assert!(terminal.has_pending_trading_request());
        terminal.pending_move_status_request = None;

        terminal.pending_move_order_contexts.insert(
            MoveOrderKey::new("BTC", 42),
            PendingMoveOrderContext::new(
                0,
                account.to_string(),
                "100",
                sensitive_string("move-agent").into_zeroizing(),
            )
            .expect("move context"),
        );
        assert!(terminal.has_pending_trading_request());
        terminal.pending_move_order_contexts.clear();

        let pending_id = terminal.add_pending_order_placement_indicator(
            account.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        assert!(pending_id.is_some());
        assert!(terminal.has_pending_trading_request());
    }

    #[test]
    fn order_signing_context_rejects_active_wallet_mismatch() {
        let mut terminal = TradingTerminal::boot().0;
        connect_test_account(&mut terminal);
        terminal.set_committed_agent_key_for_test("agent-key");
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        terminal.accounts[0].wallet_address = OTHER_ACCOUNT.to_string();

        assert!(terminal.order_signing_context().is_none());
        assert_eq!(
            terminal.order_status.as_ref(),
            Some(&(
                "Connected wallet no longer matches the active account; reconnect before trading"
                    .to_string(),
                true
            ))
        );
    }

    #[test]
    fn order_signing_context_accepts_matching_active_wallet() {
        let mut terminal = TradingTerminal::boot().0;
        connect_test_account(&mut terminal);
        terminal.set_committed_agent_key_for_test("  agent-key  ");

        let (key, account_address) = terminal
            .order_signing_context()
            .expect("matching context should trade");

        assert_eq!(key.as_str(), "agent-key");
        assert_eq!(account_address, TEST_ACCOUNT);
    }

    #[test]
    fn checked_order_signing_account_rejects_active_wallet_mismatch() {
        let mut terminal = TradingTerminal::boot().0;
        connect_test_account(&mut terminal);
        terminal.set_committed_agent_key_for_test("agent-key");
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        terminal.accounts[0].wallet_address = OTHER_ACCOUNT.to_string();

        assert!(terminal.checked_order_signing_account().is_none());
        assert_eq!(
            terminal.order_status.as_ref(),
            Some(&(
                "Connected wallet no longer matches the active account; reconnect before trading"
                    .to_string(),
                true
            ))
        );
    }

    #[test]
    fn checked_order_signing_account_accepts_matching_active_wallet() {
        let mut terminal = TradingTerminal::boot().0;
        connect_test_account(&mut terminal);
        terminal.set_committed_agent_key_for_test("  agent-key  ");

        let account_address = terminal
            .checked_order_signing_account()
            .expect("matching context should be available");

        assert_eq!(account_address, TEST_ACCOUNT);
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MoveOrderKey {
    coin: String,
    oid: u64,
}

impl MoveOrderKey {
    pub(crate) fn new(coin: impl Into<String>, oid: u64) -> Self {
        Self {
            coin: coin.into(),
            oid,
        }
    }

    pub(crate) fn coin(&self) -> &str {
        &self.coin
    }
}

#[derive(Clone)]
pub(crate) struct PendingMoveOrderContext {
    request_id: u64,
    account_address: String,
    expected_price: String,
    agent_key: CapturedAgentKey,
}

impl PendingMoveOrderContext {
    /// Captures the dispatch identity and prepared target for one modify attempt
    /// so a result cannot silently switch account/key, settle a later attempt on
    /// the same OID, or reconcile against presentation-only price state.
    pub(crate) fn new(
        request_id: u64,
        account_address: impl Into<String>,
        expected_price: impl Into<String>,
        agent_key: Zeroizing<String>,
    ) -> Result<Self, MoveOrderContextError> {
        let Some(agent_key) = CapturedAgentKey::new(agent_key) else {
            return Err(MoveOrderContextError::MissingAgentKey);
        };

        Ok(Self {
            request_id,
            account_address: account_address.into(),
            expected_price: expected_price.into(),
            agent_key,
        })
    }

    pub(crate) fn request_id(&self) -> u64 {
        self.request_id
    }

    pub(crate) fn expected_price(&self) -> &str {
        &self.expected_price
    }

    pub(crate) fn replacement_agent_key(
        &self,
        current_account: Option<&str>,
    ) -> Result<Zeroizing<String>, MoveOrderContextError> {
        match current_account {
            Some(current) => {
                let current = current.trim();
                if current.is_empty() || current != self.account_address {
                    Err(MoveOrderContextError::AccountChanged)
                } else {
                    Ok(self.agent_key.clone_for_task())
                }
            }
            _ => Err(MoveOrderContextError::AccountChanged),
        }
    }

    pub(crate) fn matches_account(&self, account_address: &str) -> bool {
        self.account_address == account_address
    }

    pub(crate) fn matches_result(&self, request_id: u64, account_address: &str) -> bool {
        self.request_id == request_id && self.matches_account(account_address)
    }
}

/// State for the right-click quick order form on a chart.
#[derive(Clone, PartialEq)]
pub(crate) struct QuickOrderForm {
    /// Price at the right-click Y coordinate (pre-filled for limit orders).
    pub(crate) price: f64,
    /// User-entered quantity string.
    pub(crate) quantity: String,
    /// True when the quantity field is USD notional, false when it is coin size.
    pub(crate) quantity_is_usd: bool,
    /// Percentage of available notional represented by the current quantity.
    pub(crate) percentage: f32,
    /// Account snapshot and pricing context used to derive `quantity` from the
    /// percentage slider.
    pub(crate) quantity_provenance: Option<QuickOrderQuantityProvenance>,
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

impl fmt::Debug for QuickOrderForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuickOrderForm")
            .field("price", &format_args!("<redacted>"))
            .field("quantity", &format_args!("<redacted>"))
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("percentage", &format_args!("<redacted>"))
            .field(
                "quantity_provenance",
                &self.quantity_provenance.as_ref().map(|_| "<redacted>"),
            )
            .field("is_limit", &self.is_limit)
            .field("click_x", &self.click_x)
            .field("click_y", &self.click_y)
            .field("chart_w", &self.chart_w)
            .field("chart_h", &self.chart_h)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct QuickOrderRecovery {
    pub(crate) chart_id: ChartId,
    pub(crate) form: QuickOrderForm,
    pub(crate) surface_id: Option<ChartSurfaceId>,
}

impl fmt::Debug for QuickOrderRecovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuickOrderRecovery")
            .field("chart_id", &self.chart_id)
            .field("form", &self.form)
            .field("surface_id", &self.surface_id)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct QuickOrderQuantityProvenance {
    pub(crate) account_address: String,
    pub(crate) account_data_revision: u64,
    pub(crate) spot_balances_revision: u64,
    pub(crate) symbol_key: String,
    pub(crate) quantity_is_usd: bool,
    pub(crate) percentage: f32,
    pub(crate) is_limit: bool,
    pub(crate) reference_price: Option<f64>,
    pub(crate) reduce_only: bool,
    pub(crate) market_universe: crate::config::MarketUniverseConfig,
}

impl fmt::Debug for QuickOrderQuantityProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuickOrderQuantityProvenance")
            .field("account_address", &"<redacted>")
            .field("account_data_revision", &self.account_data_revision)
            .field("spot_balances_revision", &self.spot_balances_revision)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("percentage", &format_args!("<redacted>"))
            .field("is_limit", &self.is_limit)
            .field(
                "reference_price",
                &self.reference_price.as_ref().map(|_| "<redacted>"),
            )
            .field("reduce_only", &self.reduce_only)
            .field("market_universe", &self.market_universe)
            .finish()
    }
}
