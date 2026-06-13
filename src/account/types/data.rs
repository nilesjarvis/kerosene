use super::{
    AccountAbstractionMode, ClearinghouseState, FundingEntry, OpenOrder, SpotClearinghouseState,
    UserFeeRates, UserFill,
};
use crate::helpers::parse_finite_number;

use std::collections::HashMap;

mod completeness;
mod fetch_scope;

pub use completeness::{AccountDataCompleteness, AccountDataSection};
pub use fetch_scope::AccountDataFetchScope;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Account Data Accessors
// ---------------------------------------------------------------------------

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
        now_ms.checked_sub(
            self.completeness
                .positions_fetched_at_ms
                .unwrap_or(self.fetched_at_ms),
        )
    }

    pub fn is_fresh_for_position_action(&self, now_ms: u64) -> bool {
        self.position_action_snapshot_age_ms(now_ms)
            .is_some_and(|age| age <= Self::POSITION_ACTION_MAX_AGE_MS)
    }

    pub fn open_order_action_snapshot_age_ms(&self, now_ms: u64) -> Option<u64> {
        now_ms.checked_sub(
            self.completeness
                .open_orders_fetched_at_ms
                .unwrap_or(self.fetched_at_ms),
        )
    }

    pub fn is_fresh_for_open_order_action(&self, now_ms: u64) -> bool {
        self.open_order_action_snapshot_age_ms(now_ms)
            .is_some_and(|age| age <= Self::POSITION_ACTION_MAX_AGE_MS)
    }

    pub fn mark_positions_fetched_at(&mut self, fetched_at_ms: u64) {
        self.completeness
            .open_orders_fetched_at_ms
            .get_or_insert(self.fetched_at_ms);
        self.completeness.positions_fetched_at_ms = Some(fetched_at_ms);
        self.fetched_at_ms = fetched_at_ms;
    }

    pub fn mark_open_orders_fetched_at(&mut self, fetched_at_ms: u64) {
        self.completeness.open_orders_fetched_at_ms = Some(fetched_at_ms);
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
        // 1. If user has interacted with this asset, the exact leverage is in asset_positions.
        if let Some(pos) = self
            .clearinghouse
            .asset_positions
            .iter()
            .find(|pos| pos.position.coin == coin)
        {
            let is_cross = pos.position.leverage.leverage_type.to_lowercase() == "cross";
            return Some((is_cross, pos.position.leverage.value, true));
        }

        // 2. Otherwise, we only know the exchange's max allowed limit for the symbol.
        // It's just for display purposes, not the actual account setting.
        for sym in symbols {
            if sym.key == coin {
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
    parse_finite_number(raw)
}
