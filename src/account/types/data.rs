use super::{
    AccountAbstractionMode, ClearinghouseState, FundingEntry, OpenOrder, SpotClearinghouseState,
    UserFeeRates, UserFill,
};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Account Data Accessors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountDataSection {
    Positions,
    OpenOrders,
    Fills,
    Funding,
    Fees,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountDataFetchScope {
    AllMarkets { hip3_dexes: Vec<String> },
    Hip3Dex { dex: String },
}

impl Default for AccountDataFetchScope {
    fn default() -> Self {
        Self::all_markets(crate::account::HIP3_DEXES.iter().copied())
    }
}

impl AccountDataFetchScope {
    pub fn all_markets(dexes: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut hip3_dexes = dexes
            .into_iter()
            .filter_map(|dex| normalized_hip3_dex(dex.as_ref()))
            .collect::<Vec<_>>();
        hip3_dexes.sort();
        hip3_dexes.dedup();
        if hip3_dexes.is_empty() {
            hip3_dexes = crate::account::HIP3_DEXES
                .iter()
                .map(|dex| (*dex).to_string())
                .collect();
        }
        Self::AllMarkets { hip3_dexes }
    }

    pub fn hip3_dex(dex: impl Into<String>) -> Self {
        normalized_hip3_dex(&dex.into())
            .map(|dex| Self::Hip3Dex { dex })
            .unwrap_or_default()
    }

    pub fn selected_hip3_dex(&self) -> Option<&str> {
        match self {
            Self::AllMarkets { .. } => None,
            Self::Hip3Dex { dex } => Some(dex.as_str()),
        }
    }

    pub fn hip3_dexes(&self, all_dexes: &[&str]) -> Vec<String> {
        match self {
            Self::AllMarkets { hip3_dexes } => {
                if hip3_dexes.is_empty() {
                    all_dexes.iter().map(|dex| (*dex).to_string()).collect()
                } else {
                    hip3_dexes.clone()
                }
            }
            Self::Hip3Dex { dex } => vec![dex.clone()],
        }
    }

    pub fn fetches_main_open_orders(&self) -> bool {
        matches!(self, Self::AllMarkets { .. })
    }

    pub fn estimated_info_weight(&self) -> u32 {
        // Hyperliquid currently weights clearinghouseState and spotClearinghouseState at 2.
        // User fills/funding have return-size adders, so use a conservative base estimate.
        let main_clearinghouse = 2;
        let spot = 2;
        let account_abstraction = 20;
        let fills = 40;
        let funding = 40;
        let fees = 20;
        let main_orders = if self.fetches_main_open_orders() {
            20
        } else {
            0
        };
        let hip3_per_dex = 2 + 20;
        main_clearinghouse
            + spot
            + account_abstraction
            + fills
            + funding
            + fees
            + main_orders
            + hip3_per_dex * self.hip3_dex_count()
    }

    pub fn automatic_refresh_interval_secs(&self) -> u64 {
        const ACCOUNT_REFRESH_WEIGHT_BUDGET_PER_MIN: u32 = 240;
        let secs = (self.estimated_info_weight() as u64 * 60)
            .div_ceil(ACCOUNT_REFRESH_WEIGHT_BUDGET_PER_MIN as u64);
        secs.clamp(45, 180)
    }

    fn hip3_dex_count(&self) -> u32 {
        match self {
            Self::AllMarkets { hip3_dexes } => hip3_dexes.len() as u32,
            Self::Hip3Dex { .. } => 1,
        }
    }
}

fn normalized_hip3_dex(dex: &str) -> Option<String> {
    let dex = dex.trim().to_ascii_lowercase();
    (!dex.is_empty()).then_some(dex)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDataCompleteness {
    pub positions_complete: bool,
    pub open_orders_complete: bool,
    pub fills_complete: bool,
    pub funding_complete: bool,
    pub fees_complete: bool,
    warnings: Vec<(AccountDataSection, String)>,
}

/// All account data fetched in one batch.
#[derive(Debug, Clone)]
pub struct AccountData {
    pub fetch_scope: AccountDataFetchScope,
    pub request_weight_estimate: u32,
    pub account_abstraction: AccountAbstractionMode,
    pub clearinghouse: ClearinghouseState,
    pub clearinghouses_by_dex: HashMap<String, ClearinghouseState>,
    pub spot: SpotClearinghouseState,
    pub open_orders: Vec<OpenOrder>,
    pub fills: Vec<UserFill>,
    /// Recent funding payments (last 7 days).
    pub funding_history: Vec<FundingEntry>,
    /// User's personalized fee rates.
    pub fee_rates: UserFeeRates,
    pub completeness: AccountDataCompleteness,
    /// Wall-clock time (milliseconds since UNIX epoch) when this snapshot was fetched.
    pub fetched_at_ms: u64,
}

impl AccountData {
    pub const POSITION_ACTION_MAX_AGE_MS: u64 = 15_000;

    pub fn position_action_snapshot_age_ms(&self, now_ms: u64) -> Option<u64> {
        now_ms.checked_sub(self.fetched_at_ms)
    }

    pub fn is_fresh_for_position_action(&self, now_ms: u64) -> bool {
        self.position_action_snapshot_age_ms(now_ms)
            .is_some_and(|age| age <= Self::POSITION_ACTION_MAX_AGE_MS)
    }

    /// Whether this account has portfolio margin enabled.
    pub fn is_portfolio_margin(&self) -> bool {
        self.spot.portfolio_margin_enabled
            || self.account_abstraction == AccountAbstractionMode::PortfolioMargin
    }

    /// Whether account-level balance metrics should ignore individual perp-dex balances.
    pub fn uses_shared_account_balance(&self) -> bool {
        self.is_portfolio_margin() || self.account_abstraction.uses_shared_account_balance()
    }

    /// Available balance after maintenance margin for USDC (token 0).
    pub fn available_after_maintenance_for_token(&self, token: u32) -> Option<f64> {
        self.spot
            .token_to_available_after_maintenance
            .as_ref()
            .and_then(|values| values.iter().find(|(entry_token, _)| *entry_token == token))
            .and_then(|(_, value)| parse_account_number(value))
    }

    /// Available spot balance for a token after spot holds are removed.
    pub fn spot_available_for_token(&self, token: u32) -> Option<f64> {
        let balance = self
            .spot
            .balances
            .iter()
            .find(|balance| balance.token == Some(token))
            .or_else(|| {
                if token == 0 {
                    self.spot
                        .balances
                        .iter()
                        .find(|balance| balance.coin == "USDC")
                } else {
                    None
                }
            })?;
        let total = parse_account_number(&balance.total)?;
        let hold = parse_account_number(&balance.hold)?;
        Some(total - hold)
    }

    /// Account-level value in USDC terms when the API reports one directly.
    pub fn account_value_usdc(&self) -> Option<f64> {
        parse_account_number(&self.clearinghouse.margin_summary.account_value)
    }

    /// Margin available for order sizing in USDC terms.
    pub fn available_margin_usdc(&self) -> Option<f64> {
        self.available_margin_for_token(0)
    }

    /// Margin available for order sizing in the requested collateral token.
    pub fn available_margin_for_token(&self, token: u32) -> Option<f64> {
        if matches!(self.account_abstraction, AccountAbstractionMode::Unknown(_)) {
            return None;
        }

        if self.is_portfolio_margin() {
            self.available_after_maintenance_for_token(token)
                .or_else(|| self.spot_available_for_token(token))
        } else if self.account_abstraction == AccountAbstractionMode::UnifiedAccount {
            self.spot_available_for_token(token)
        } else if matches!(
            self.account_abstraction,
            AccountAbstractionMode::Default | AccountAbstractionMode::DexAbstraction
        ) {
            if token != 0 {
                return self.spot_available_for_token(token);
            }
            match (self.withdrawable(), self.spot_available_for_token(0)) {
                (Some(withdrawable), Some(spot_available)) => {
                    Some(withdrawable.max(spot_available))
                }
                (Some(withdrawable), None) => Some(withdrawable),
                (None, Some(spot_available)) => Some(spot_available),
                (None, None) => None,
            }
        } else {
            self.withdrawable()
        }
    }

    /// Withdrawable amount.
    pub fn withdrawable(&self) -> Option<f64> {
        parse_account_number(&self.clearinghouse.withdrawable)
    }

    /// Returns (is_cross, leverage, is_account_setting).
    pub fn get_leverage_for(
        &self,
        coin: &str,
        symbols: &[crate::api::ExchangeSymbol],
    ) -> Option<(bool, u32, bool)> {
        let original_coin = coin;
        let mut search_coin = coin;
        if let Some((_, suffix)) = coin.split_once(':') {
            search_coin = suffix;
        }

        // 1. If user has interacted with this asset, the exact leverage is in asset_positions.
        if let Some(pos) = self
            .clearinghouse
            .asset_positions
            .iter()
            .find(|pos| pos.position.coin == original_coin)
            .or_else(|| {
                self.clearinghouse
                    .asset_positions
                    .iter()
                    .find(|pos| pos.position.coin == search_coin)
            })
        {
            let is_cross = pos.position.leverage.leverage_type.to_lowercase() == "cross";
            return Some((is_cross, pos.position.leverage.value, true));
        }

        // 2. Otherwise, we only know the exchange's max allowed limit for the symbol.
        // It's just for display purposes, not the actual account setting.
        for sym in symbols {
            if sym.key == original_coin {
                if !matches!(sym.market_type, crate::api::MarketType::Perp) {
                    return None; // No leverage for spot or outcome markets
                }
                let is_cross = !sym.only_isolated;
                return Some((is_cross, sym.max_leverage, false));
            }
        }

        None
    }
}

fn parse_account_number(raw: &str) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

impl Default for AccountDataCompleteness {
    fn default() -> Self {
        Self {
            positions_complete: true,
            open_orders_complete: true,
            fills_complete: true,
            funding_complete: true,
            fees_complete: true,
            warnings: Vec::new(),
        }
    }
}

impl AccountDataCompleteness {
    pub fn is_complete(&self) -> bool {
        self.positions_complete
            && self.open_orders_complete
            && self.fills_complete
            && self.funding_complete
            && self.fees_complete
    }

    pub fn mark_incomplete(&mut self, section: AccountDataSection, warning: impl Into<String>) {
        match section {
            AccountDataSection::Positions => self.positions_complete = false,
            AccountDataSection::OpenOrders => self.open_orders_complete = false,
            AccountDataSection::Fills => self.fills_complete = false,
            AccountDataSection::Funding => self.funding_complete = false,
            AccountDataSection::Fees => self.fees_complete = false,
        }

        let warning = warning.into();
        if !warning.is_empty()
            && !self
                .warnings
                .iter()
                .any(|existing| existing == &(section, warning.clone()))
        {
            self.warnings.push((section, warning));
        }
    }

    pub fn warning_summary(&self) -> Option<String> {
        if self.warnings.is_empty() {
            None
        } else {
            Some(format!(
                "Partial account data: {}",
                self.warning_messages().join("; ")
            ))
        }
    }

    pub fn section_warning(&self, section: AccountDataSection) -> Option<String> {
        let complete = match section {
            AccountDataSection::Positions => self.positions_complete,
            AccountDataSection::OpenOrders => self.open_orders_complete,
            AccountDataSection::Fills => self.fills_complete,
            AccountDataSection::Funding => self.funding_complete,
            AccountDataSection::Fees => self.fees_complete,
        };
        if complete {
            return None;
        }

        let label = match section {
            AccountDataSection::Positions => "Positions",
            AccountDataSection::OpenOrders => "Open orders",
            AccountDataSection::Fills => "Trade history",
            AccountDataSection::Funding => "Funding history",
            AccountDataSection::Fees => "Fee rates",
        };
        let warnings = self
            .warnings
            .iter()
            .filter_map(|(warning_section, warning)| {
                (*warning_section == section).then_some(warning.as_str())
            })
            .collect::<Vec<_>>();
        let detail = if warnings.is_empty() {
            "refresh account data before relying on this section".to_string()
        } else {
            warnings.join("; ")
        };

        Some(format!("{label} may be incomplete: {detail}"))
    }

    fn warning_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        for (_, warning) in &self.warnings {
            if !messages.iter().any(|existing| existing == warning) {
                messages.push(warning.clone());
            }
        }
        messages
    }
}
