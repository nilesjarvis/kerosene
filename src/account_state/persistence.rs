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
