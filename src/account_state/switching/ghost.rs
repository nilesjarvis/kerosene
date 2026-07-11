use crate::app_state::TradingTerminal;
use crate::config::{self, AccountProfile};
use crate::message::Message;

use iced::Task;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Ghost Wallet Switching
// ---------------------------------------------------------------------------

fn find_ghost_account_index(
    accounts: &[AccountProfile],
    ghost_account_secret_ids: &HashSet<String>,
    address: &str,
) -> Option<usize> {
    accounts.iter().position(|profile| {
        ghost_account_secret_ids.contains(&profile.secret_id)
            && TradingTerminal::normalize_wallet_address(&profile.wallet_address)
                .is_some_and(|profile_address| profile_address == address)
    })
}

impl TradingTerminal {
    pub(crate) fn find_ghost_account_by_wallet_address(&self, address: &str) -> Option<usize> {
        find_ghost_account_index(&self.accounts, &self.ghost_account_secret_ids, address)
    }

    pub(crate) fn ghost_account_name(&self, address: &str) -> String {
        let display = self.wallet_display(address);
        format!("Ghost: {}", display.primary)
    }

    pub(crate) fn ghost_wallet_task(&mut self, address: String) -> Task<Message> {
        if self.account_change_blocked_by_pending_trading_request("switching accounts") {
            return Task::none();
        }

        let Some(address) = Self::normalize_wallet_address(&address) else {
            self.push_toast("Invalid wallet address".to_string(), true);
            return Task::none();
        };

        if let Some(index) = self.find_ghost_account_by_wallet_address(&address) {
            return self.switch_account_task(index);
        }

        if self.account_change_blocked_by_active_chase("switching accounts") {
            return Task::none();
        }
        if self.account_change_blocked_by_uncertain_twap("switching accounts") {
            return Task::none();
        }

        let profile = AccountProfile {
            secret_id: config::new_secret_id(),
            name: self.ghost_account_name(&address),
            wallet_address: address,
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        };
        let secret_id = profile.secret_id.clone();
        self.accounts.push(profile);
        self.ghost_account_secret_ids.insert(secret_id);
        self.switch_account_task(self.accounts.len() - 1)
    }

    pub(crate) fn forget_ghost_account_task(&mut self, index: usize) -> Task<Message> {
        let Some(profile) = self.accounts.get(index) else {
            return Task::none();
        };
        let secret_id = profile.secret_id.clone();
        if !self.ghost_account_secret_ids.contains(&secret_id) {
            return Task::none();
        }

        if self.account_change_blocked_by_active_automation("forgetting a ghost wallet") {
            return Task::none();
        }

        if self.account_change_blocked_by_pending_trading_request("forgetting a ghost wallet") {
            return Task::none();
        }

        let selected_cluster_profile_removed =
            self.selected_wallet_cluster_uses_profile(&secret_id);
        let was_active = self.active_account_index == index;
        if was_active {
            self.journal.switch_active_account(None);
            self.show_hidden_positions = false;
        }
        self.journal.account_states.remove(&secret_id);
        self.hidden_positions_by_account.remove(&secret_id);
        self.accounts.remove(index);
        self.ghost_account_secret_ids.remove(&secret_id);
        if selected_cluster_profile_removed {
            self.rotate_wallet_cluster_user_data_streams();
        }

        if self.active_account_index > index {
            self.active_account_index -= 1;
        }

        if was_active {
            if let Some(fallback_index) = self.fallback_persisted_account_index() {
                self.active_account_index = self.accounts.len();
                self.switch_account_task(fallback_index)
            } else {
                self.active_account_index = 0;
                self.journal
                    .switch_active_account(self.active_journal_account_key());
                Task::done(Message::DisconnectWallet)
            }
        } else {
            self.persist_config();
            Task::none()
        }
    }
}

#[cfg(test)]
mod tests;
