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
pub(crate) use hud::{HudOrderRequest, HudOrderSide, HudOrderType};
pub(crate) use position_actions::{NukePlan, reject_if_positions_incomplete_for_action};
pub(crate) use quick_order::QuickOrderSubmissionSnapshot;
pub(crate) use sizing::order_size_from_quantity_input;
pub(crate) use submit::{TicketOrderPlaceIntent, TicketOrderSubmissionSnapshot};

#[cfg(test)]
pub(crate) use position_actions::{NukePositionOrder, NukeSkipReason};

use crate::account::{AccountData, OpenOrder};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::config;
use crate::signing::{CapturedAgentKey, ChaseOrder};
use std::fmt;
use zeroize::Zeroizing;

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

pub(in crate::order_execution) fn order_account_addresses_match(left: &str, right: &str) -> bool {
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
            || self.pending_one_shot_status_request.is_some()
            || !self.pending_move_order_contexts.is_empty()
            || self.has_pending_order_indicator_for_connected_account()
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

#[cfg(test)]
mod tests {
    use super::{
        MoveOrderKey, OneShotPlacementContext, OrderSurface, PendingLeverageUpdateContext,
        PendingMoveOrderContext, PendingNukeExecution, PendingOrderAction,
        QuickOrderQuantityProvenance,
    };
    use crate::account::{
        AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
        SpotClearinghouseState,
    };
    use crate::app_state::{TradingTerminal, sensitive_string};
    use crate::config::AccountProfile;
    use crate::order_update::PendingOneShotStatusRequest;
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
            symbol_key: "BTC".to_string(),
            quantity_is_usd: true,
            percentage: 25.0,
            is_limit: false,
            reference_price: Some(100.0),
            reduce_only: false,
            market_universe: crate::config::MarketUniverseConfig::default(),
        };

        let rendered = format!("{provenance:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(TEST_ACCOUNT));
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

        terminal.pending_one_shot_status_request = Some(PendingOneShotStatusRequest::new(
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
        terminal.pending_one_shot_status_request = None;

        terminal.pending_move_order_contexts.insert(
            MoveOrderKey::new("BTC", 42),
            PendingMoveOrderContext::new(
                account.to_string(),
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
    account_address: String,
    agent_key: CapturedAgentKey,
}

impl PendingMoveOrderContext {
    /// Captures the trading identity used to cancel an order so the replacement
    /// cannot silently switch to a different account/key before placement.
    pub(crate) fn new(
        account_address: impl Into<String>,
        agent_key: Zeroizing<String>,
    ) -> Result<Self, MoveOrderContextError> {
        let Some(agent_key) = CapturedAgentKey::new(agent_key) else {
            return Err(MoveOrderContextError::MissingAgentKey);
        };

        Ok(Self {
            account_address: account_address.into(),
            agent_key,
        })
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
}

/// State for the right-click quick order form on a chart.
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuickOrderRecovery {
    pub(crate) chart_id: ChartId,
    pub(crate) form: QuickOrderForm,
    pub(crate) surface_id: Option<ChartSurfaceId>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct QuickOrderQuantityProvenance {
    pub(crate) account_address: String,
    pub(crate) account_data_revision: u64,
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
            .field("symbol_key", &self.symbol_key)
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("percentage", &self.percentage)
            .field("is_limit", &self.is_limit)
            .field("reference_price", &self.reference_price)
            .field("reduce_only", &self.reduce_only)
            .field("market_universe", &self.market_universe)
            .finish()
    }
}
