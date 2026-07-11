use crate::app_state::TradingTerminal;
use crate::config::AccountProfile;
use crate::journal;

use std::collections::{HashMap, HashSet};

impl TradingTerminal {
    pub(crate) fn persisted_accounts_from(
        accounts: &[AccountProfile],
        ghost_account_secret_ids: &HashSet<String>,
    ) -> Vec<AccountProfile> {
        accounts
            .iter()
            .filter(|profile| !ghost_account_secret_ids.contains(&profile.secret_id))
            .cloned()
            .collect()
    }

    pub(crate) fn persisted_active_account_index_from_ids(
        persisted_accounts: &[AccountProfile],
        active_secret_id: Option<&str>,
        fallback_secret_id: Option<&str>,
    ) -> usize {
        active_secret_id
            .or(fallback_secret_id)
            .and_then(|secret_id| {
                persisted_accounts
                    .iter()
                    .position(|profile| profile.secret_id == secret_id)
            })
            .unwrap_or(0)
    }

    pub(crate) fn persisted_accounts_snapshot(&self) -> Vec<AccountProfile> {
        Self::persisted_accounts_from(&self.accounts, &self.ghost_account_secret_ids)
    }

    pub(crate) fn persisted_accounts_with_active_agent_key(
        &self,
        agent_key: &str,
    ) -> Option<(Vec<AccountProfile>, usize)> {
        // Construct the storage snapshot directly with the draft key so the
        // previous committed signing key is never cloned into staging.
        let active_index = self.active_account_index;
        let active_profile = self.accounts.get(active_index)?;
        if self
            .ghost_account_secret_ids
            .contains(&active_profile.secret_id)
        {
            return None;
        }

        let mut persisted_accounts = Vec::with_capacity(self.accounts.len());
        let mut persisted_active_index = None;
        for (index, profile) in self.accounts.iter().enumerate() {
            if self.ghost_account_secret_ids.contains(&profile.secret_id) {
                continue;
            }

            if index == active_index {
                persisted_active_index = Some(persisted_accounts.len());
                persisted_accounts.push(AccountProfile {
                    secret_id: profile.secret_id.clone(),
                    name: profile.name.clone(),
                    wallet_address: profile.wallet_address.clone(),
                    agent_key: agent_key.to_string().into(),
                    hydromancer_api_key: profile.hydromancer_api_key.clone(),
                });
            } else {
                persisted_accounts.push(profile.clone());
            }
        }

        Some((persisted_accounts, persisted_active_index?))
    }

    pub(crate) fn persisted_journal_entries_by_account_from(
        journal: &journal::JournalState,
        ghost_account_secret_ids: &HashSet<String>,
    ) -> HashMap<String, HashMap<String, journal::JournalNote>> {
        journal
            .entries_by_account_snapshot()
            .into_iter()
            .filter(|(key, _)| !ghost_account_secret_ids.contains(key))
            .collect()
    }

    pub(crate) fn persisted_journal_entries_by_account(
        &self,
    ) -> HashMap<String, HashMap<String, journal::JournalNote>> {
        Self::persisted_journal_entries_by_account_from(
            &self.journal,
            &self.ghost_account_secret_ids,
        )
    }

    pub(crate) fn persisted_hidden_positions_by_account(
        &self,
        persisted_accounts: &[AccountProfile],
    ) -> HashMap<String, Vec<String>> {
        let persisted_ids: HashSet<&str> = persisted_accounts
            .iter()
            .map(|profile| profile.secret_id.as_str())
            .collect();

        self.hidden_positions_by_account
            .iter()
            .filter(|(account_key, hidden)| {
                persisted_ids.contains(account_key.as_str()) && !hidden.is_empty()
            })
            .map(|(account_key, hidden)| {
                let mut coins: Vec<String> = hidden.iter().cloned().collect();
                coins.sort();
                (account_key.clone(), coins)
            })
            .collect()
    }

    pub(crate) fn active_journal_account_key(&self) -> Option<String> {
        self.accounts
            .get(self.active_account_index)
            .map(|profile| profile.secret_id.clone())
    }

    pub(crate) fn journal_active_account_is_ghost(&self) -> bool {
        self.journal
            .active_account_key
            .as_ref()
            .is_some_and(|key| self.ghost_account_secret_ids.contains(key))
    }

    pub(crate) fn active_persisted_account_secret_id(&self) -> Option<&str> {
        self.accounts
            .get(self.active_account_index)
            .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
            .map(|profile| profile.secret_id.as_str())
    }

    pub(crate) fn persisted_active_account_index(
        &self,
        persisted_accounts: &[AccountProfile],
    ) -> usize {
        Self::persisted_active_account_index_from_ids(
            persisted_accounts,
            self.active_persisted_account_secret_id(),
            self.last_persisted_active_account_secret_id.as_deref(),
        )
    }

    pub(crate) fn account_index_is_ghost(&self, index: usize) -> bool {
        self.accounts
            .get(index)
            .is_some_and(|profile| self.ghost_account_secret_ids.contains(&profile.secret_id))
    }

    pub(crate) fn active_account_is_ghost(&self) -> bool {
        self.account_index_is_ghost(self.active_account_index)
    }
}
