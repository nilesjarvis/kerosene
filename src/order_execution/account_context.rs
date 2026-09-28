use crate::account::AccountData;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::signing::CapturedAgentKey;

#[cfg(test)]
mod tests;

pub(crate) fn order_account_addresses_match(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();
    !left.is_empty() && !right.is_empty() && left.eq_ignore_ascii_case(right)
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

    pub(crate) fn has_active_committed_agent_key(&self) -> bool {
        self.accounts
            .get(self.active_account_index)
            .is_some_and(|profile| !profile.agent_key.trim().is_empty())
    }

    pub(crate) fn checked_order_signing_account(&mut self) -> Option<String> {
        self.captured_order_signing_context()
            .map(|(_, account_address)| account_address)
    }

    /// Capture the effective trading target with the committed key. Reads and
    /// reconciliation continue to use wallet_address for both account kinds.
    pub(crate) fn capture_profile_signing_key(
        profile: &config::AccountProfile,
    ) -> Result<CapturedAgentKey, String> {
        let address = Self::normalize_wallet_address(&profile.wallet_address)
            .ok_or_else(|| "Trading profile has an invalid account address".to_string())?;
        let vault_address = if let Some(master_address) = &profile.master_address {
            let master = Self::normalize_wallet_address(master_address)
                .ok_or_else(|| "Subaccount profile has an invalid parent address".to_string())?;
            if master == address {
                return Err("Subaccount address must differ from its parent address".to_string());
            }
            Some(address.as_str())
        } else {
            None
        };
        CapturedAgentKey::for_account(profile.agent_key.clone(), vault_address)
    }

    pub(crate) fn order_signing_context(&mut self) -> Option<(CapturedAgentKey, String)> {
        self.captured_order_signing_context()
    }

    pub(crate) fn captured_order_signing_context(&mut self) -> Option<(CapturedAgentKey, String)> {
        if !self.has_active_committed_agent_key() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        }
        if self.active_account_is_ghost() {
            self.order_status = Some(("Watch-only accounts cannot sign orders".into(), true));
            return None;
        }
        let Some(account_address) = self.connected_order_account_address() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return None;
        };
        if self.reject_mismatched_trading_context(&account_address) {
            return None;
        }

        let profile = self.accounts.get(self.active_account_index)?;
        match Self::capture_profile_signing_key(profile) {
            Ok(key) => Some((key, account_address)),
            Err(error) => {
                self.order_status = Some((error, true));
                None
            }
        }
    }
}
