use super::{
    ClearinghouseState, FundingEntry, OpenOrder, SpotClearinghouseState, UserFeeRates, UserFill,
};

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
    pub clearinghouse: ClearinghouseState,
    pub spot: SpotClearinghouseState,
    pub open_orders: Vec<OpenOrder>,
    pub fills: Vec<UserFill>,
    /// Recent funding payments (last 7 days).
    pub funding_history: Vec<FundingEntry>,
    /// User's personalized fee rates.
    pub fee_rates: UserFeeRates,
    pub completeness: AccountDataCompleteness,
}

impl AccountData {
    /// Whether this account has portfolio margin enabled.
    pub fn is_portfolio_margin(&self) -> bool {
        self.spot.portfolio_margin_enabled
    }

    /// Available balance after maintenance margin for USDC (token 0).
    pub fn available_after_maintenance_usdc(&self) -> Option<f64> {
        self.spot
            .token_to_available_after_maintenance
            .as_ref()
            .and_then(|values| values.iter().find(|(token, _)| *token == 0))
            .and_then(|(_, value)| parse_account_number(value))
    }

    /// Margin available for order sizing in USDC terms.
    pub fn available_margin_usdc(&self) -> Option<f64> {
        if self.is_portfolio_margin() {
            self.available_after_maintenance_usdc()
        } else {
            self.withdrawable()
        }
    }

    /// Total margin used from perp margin summary.
    pub fn total_margin_used(&self) -> Option<f64> {
        parse_account_number(&self.clearinghouse.margin_summary.total_margin_used)
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
        for pos in &self.clearinghouse.asset_positions {
            if pos.position.coin == search_coin {
                let is_cross = pos.position.leverage.leverage_type.to_lowercase() == "cross";
                return Some((is_cross, pos.position.leverage.value, true));
            }
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
