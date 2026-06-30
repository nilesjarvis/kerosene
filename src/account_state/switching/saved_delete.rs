use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
use crate::journal::JournalAccountState;
use crate::message::Message;

use iced::Task;
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Saved Account Deletion
// ---------------------------------------------------------------------------

struct SavedAccountDeleteRollback {
    accounts: Vec<config::AccountProfile>,
    pending_keychain_profile_deletions: Vec<String>,
    active_account_index: usize,
    ghost_account_secret_ids: HashSet<String>,
    last_persisted_active_account_secret_id: Option<String>,
    hidden_positions_by_account: HashMap<String, HashSet<String>>,
    show_hidden_positions: bool,
    journal_active_account_key: Option<String>,
    journal_account_states: HashMap<String, JournalAccountState>,
    journal_active_state: JournalAccountState,
    encrypted_secrets: Option<config::EncryptedSecretsConfig>,
    encrypted_secrets_unlocked: bool,
    secret_migration_save_blocked: bool,
    secret_store_status: Option<(String, bool)>,
}

impl SavedAccountDeleteRollback {
    fn capture(terminal: &TradingTerminal) -> Self {
        Self {
            accounts: terminal.accounts.clone(),
            pending_keychain_profile_deletions: terminal.pending_keychain_profile_deletions.clone(),
            active_account_index: terminal.active_account_index,
            ghost_account_secret_ids: terminal.ghost_account_secret_ids.clone(),
            last_persisted_active_account_secret_id: terminal
                .last_persisted_active_account_secret_id
                .clone(),
            hidden_positions_by_account: terminal.hidden_positions_by_account.clone(),
            show_hidden_positions: terminal.show_hidden_positions,
            journal_active_account_key: terminal.journal.active_account_key.clone(),
            journal_account_states: terminal.journal.account_states.clone(),
            journal_active_state: terminal.journal.snapshot_active_account_state(),
            encrypted_secrets: terminal.encrypted_secrets.clone(),
            encrypted_secrets_unlocked: terminal.encrypted_secrets_unlocked,
            secret_migration_save_blocked: terminal.secret_migration_save_blocked,
            secret_store_status: terminal.secret_store_status.clone(),
        }
    }

    fn restore(self, terminal: &mut TradingTerminal) {
        terminal.accounts = self.accounts;
        terminal.pending_keychain_profile_deletions = self.pending_keychain_profile_deletions;
        terminal.active_account_index = self.active_account_index;
        terminal.ghost_account_secret_ids = self.ghost_account_secret_ids;
        terminal.last_persisted_active_account_secret_id =
            self.last_persisted_active_account_secret_id;
        terminal.hidden_positions_by_account = self.hidden_positions_by_account;
        terminal.show_hidden_positions = self.show_hidden_positions;
        terminal.journal.active_account_key = self.journal_active_account_key;
        terminal.journal.account_states = self.journal_account_states;
        terminal
            .journal
            .restore_active_account_state(self.journal_active_state);
        terminal.encrypted_secrets = self.encrypted_secrets;
        terminal.encrypted_secrets_unlocked = self.encrypted_secrets_unlocked;
        terminal.secret_migration_save_blocked = self.secret_migration_save_blocked;
        terminal.secret_store_status = self.secret_store_status;
    }
}

impl TradingTerminal {
    fn secret_payload_after_saved_account_removal(
        &self,
        removed_secret_id: &str,
    ) -> config::SecretPayload {
        let accounts: Vec<_> = self
            .persisted_accounts_snapshot()
            .into_iter()
            .filter(|profile| profile.secret_id != removed_secret_id)
            .collect();
        let x_access_token = self.x_feed.access_token_for_secret();
        config::SecretPayload::from_credentials_with_x(
            &accounts,
            &self.hydromancer_api_key,
            &self.hyperdash_api_key,
            x_access_token.as_str(),
        )
    }

    fn stage_pending_keychain_profile_deletion(&mut self, secret_id: &str) {
        if !self
            .pending_keychain_profile_deletions
            .iter()
            .any(|pending| pending == secret_id)
        {
            self.pending_keychain_profile_deletions
                .push(secret_id.to_string());
        }
    }

    fn clear_pending_keychain_profile_deletion(&mut self, secret_id: &str) {
        self.pending_keychain_profile_deletions
            .retain(|pending| pending != secret_id);
    }

    /// Adjust the active-account index after the account at `removed_index`
    /// has been removed from the `accounts` vec, so the same logical account
    /// stays active when one before it was deleted.
    pub(super) fn adjust_active_index_after_removal(
        active_index: usize,
        removed_index: usize,
    ) -> usize {
        if active_index > removed_index {
            active_index - 1
        } else {
            active_index
        }
    }

    pub(crate) fn delete_saved_account_task(&mut self, index: usize) -> Task<Message> {
        self.delete_saved_account_task_with_encrypted_prepare(index, |terminal, payload| {
            terminal.encrypted_secret_blob_for_payload(payload)
        })
    }

    fn delete_saved_account_task_with_encrypted_prepare(
        &mut self,
        index: usize,
        prepare_encrypted_blob: impl FnOnce(
            &mut Self,
            &config::SecretPayload,
        ) -> Option<config::EncryptedSecretsConfig>,
    ) -> Task<Message> {
        self.delete_saved_account_task_with_hooks(
            index,
            prepare_encrypted_blob,
            config::save_config,
            config::clear_profile_secrets,
        )
    }

    fn delete_saved_account_task_with_hooks(
        &mut self,
        index: usize,
        prepare_encrypted_blob: impl FnOnce(
            &mut Self,
            &config::SecretPayload,
        ) -> Option<config::EncryptedSecretsConfig>,
        mut save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
        clear_profile_secrets: impl FnOnce(&config::AccountProfile) -> Result<(), String>,
    ) -> Task<Message> {
        self.account_picker_open = false;
        self.account_picker_rename_index = None;

        if self.account_change_blocked_by_pending_trading_request("deleting an account") {
            return Task::none();
        }

        let Some(profile_snapshot) = self.accounts.get(index).cloned() else {
            return Task::none();
        };

        if self.account_change_blocked_by_active_automation("deleting an account") {
            return Task::none();
        }

        // Ghost sessions have their own forget path; this entry point is for
        // saved profiles, which need keychain/encrypted-blob cleanup that
        // ghost accounts never accumulated.
        if self
            .ghost_account_secret_ids
            .contains(&profile_snapshot.secret_id)
        {
            return self.forget_ghost_account_task(index);
        }

        if self.encrypted_credentials_locked() {
            self.show_unlock_credentials_popup = true;
            self.push_toast(
                "Unlock encrypted credentials before deleting an account".to_string(),
                true,
            );
            return Task::none();
        }

        if self.secret_storage_mode == config::CredentialStorageMode::EncryptedConfig
            && !self.encrypted_password_is_ready()
        {
            self.push_toast(
                "Enter the encrypted credential password before deleting an account".to_string(),
                true,
            );
            return Task::none();
        }

        let secret_id = profile_snapshot.secret_id.clone();
        let account_label = profile_snapshot.name.clone();
        let was_active = self.active_account_index == index;
        let os_keychain_delete =
            self.secret_storage_mode == config::CredentialStorageMode::OsKeychain;
        let encrypted_config_delete =
            self.secret_storage_mode == config::CredentialStorageMode::EncryptedConfig;
        let durable_config_delete = os_keychain_delete || encrypted_config_delete;
        let mut rollback = durable_config_delete.then(|| SavedAccountDeleteRollback::capture(self));
        let staged_encrypted_blob = if encrypted_config_delete {
            let payload = self.secret_payload_after_saved_account_removal(&secret_id);
            let Some(encrypted) = prepare_encrypted_blob(self, &payload) else {
                self.push_toast(
                    "Could not delete account: encrypted credential cleanup failed".to_string(),
                    true,
                );
                return Task::none();
            };
            Some(encrypted)
        } else {
            None
        };

        if os_keychain_delete {
            self.stage_pending_keychain_profile_deletion(&secret_id);
        }

        if was_active {
            self.journal.switch_active_account(None);
            self.show_hidden_positions = false;
        }
        self.journal.account_states.remove(&secret_id);
        self.hidden_positions_by_account.remove(&secret_id);
        self.accounts.remove(index);
        self.ghost_account_secret_ids.remove(&secret_id);
        if self.last_persisted_active_account_secret_id.as_deref() == Some(secret_id.as_str()) {
            self.last_persisted_active_account_secret_id = None;
        }

        self.active_account_index =
            Self::adjust_active_index_after_removal(self.active_account_index, index);

        if let Some(encrypted) = staged_encrypted_blob {
            self.store_encrypted_secret_blob(encrypted, "Deleted account from encrypted config");
        }

        let durable_save_active_index = if durable_config_delete && was_active {
            self.fallback_persisted_account_index()
        } else {
            None
        };
        let active_index_before_durable_save =
            durable_save_active_index.map(|_| self.active_account_index);
        if let Some(fallback_index) = durable_save_active_index {
            self.active_account_index = fallback_index;
        }

        if durable_config_delete
            && let Err(error) = self.persist_config_immediately_with(&mut save_config)
        {
            let error = redact_sensitive_response_text(&error);
            if let Some(rollback) = rollback.take() {
                rollback.restore(self);
            }
            self.push_toast(
                format!("Could not delete '{account_label}': config save failed: {error}"),
                true,
            );
            return Task::none();
        }

        let mut deletion_status_toast_shown = false;
        if os_keychain_delete {
            match clear_profile_secrets(&profile_snapshot) {
                Ok(()) => {
                    self.clear_pending_keychain_profile_deletion(&secret_id);
                    if let Err(error) = self.persist_config_immediately_with(&mut save_config) {
                        let error = redact_sensitive_response_text(&error);
                        self.secret_store_status = Some((
                            "Deleted account, but credential cleanup state could not be saved; cleanup may retry on next startup".to_string(),
                            true,
                        ));
                        self.push_toast(
                            format!("Deleted account, but cleanup state save failed: {error}"),
                            true,
                        );
                        deletion_status_toast_shown = true;
                    }
                }
                Err(_) => {
                    self.secret_store_status = Some((
                        "Deleted account, but OS keychain cleanup failed; cleanup will retry on next startup".to_string(),
                        true,
                    ));
                    self.push_toast(
                        "Deleted account, but OS keychain cleanup failed and will retry"
                            .to_string(),
                        true,
                    );
                    deletion_status_toast_shown = true;
                }
            }
        }

        if let Some(active_index) = active_index_before_durable_save {
            self.active_account_index = active_index;
        }

        if !deletion_status_toast_shown {
            self.push_toast(format!("Deleted account: {account_label}"), false);
        }

        if was_active {
            if let Some(fallback_index) = self.fallback_persisted_account_index() {
                self.active_account_index = self.accounts.len();
                self.switch_account_task(fallback_index)
            } else {
                self.active_account_index = 0;
                self.journal
                    .switch_active_account(self.active_journal_account_key());
                self.persist_config();
                Task::done(Message::DisconnectWallet)
            }
        } else {
            self.persist_config();
            Task::none()
        }
    }
}
