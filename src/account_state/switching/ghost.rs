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
        if self.pending_order_action.is_some() {
            self.push_toast(
                "Wait for the pending order request before switching accounts".to_string(),
                true,
            );
            return Task::none();
        }

        let Some(address) = Self::normalize_wallet_address(&address) else {
            self.push_toast("Invalid wallet address".to_string(), true);
            return Task::none();
        };

        if let Some(index) = self.find_ghost_account_by_wallet_address(&address) {
            return self.switch_account_task(index);
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

        if self.account_change_blocked_by_active_chase("forgetting a ghost wallet") {
            return Task::none();
        }

        let was_active = self.active_account_index == index;
        if was_active {
            self.journal.switch_active_account(None);
            self.show_hidden_positions = false;
        }
        self.journal.account_states.remove(&secret_id);
        self.hidden_positions_by_account.remove(&secret_id);
        self.accounts.remove(index);
        self.ghost_account_secret_ids.remove(&secret_id);

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
mod tests {
    use super::*;

    use zeroize::Zeroizing;

    const WALLET: &str = "0x1111111111111111111111111111111111111111";

    fn account(
        secret_id: &str,
        name: &str,
        wallet_address: &str,
        agent_key: &str,
    ) -> AccountProfile {
        AccountProfile {
            secret_id: secret_id.to_string(),
            name: name.to_string(),
            wallet_address: wallet_address.to_string(),
            agent_key: Zeroizing::new(agent_key.to_string()),
            hydromancer_api_key: Zeroizing::new(String::new()),
        }
    }

    #[test]
    fn ghost_wallet_lookup_ignores_saved_trading_profile_with_same_address() {
        let accounts = vec![account("saved", "Saved", WALLET, "agent-key")];
        let ghost_account_secret_ids = HashSet::new();

        assert_eq!(
            find_ghost_account_index(&accounts, &ghost_account_secret_ids, WALLET),
            None
        );
    }

    #[test]
    fn ghost_wallet_lookup_reuses_existing_ghost_profile() {
        let accounts = vec![
            account("saved", "Saved", WALLET, "agent-key"),
            account("ghost", "Ghost", WALLET, ""),
        ];
        let ghost_account_secret_ids = HashSet::from(["ghost".to_string()]);

        assert_eq!(
            find_ghost_account_index(&accounts, &ghost_account_secret_ids, WALLET),
            Some(1)
        );
    }
}
