use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;

use iced::Task;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Saved Account Deletion
// ---------------------------------------------------------------------------

impl TradingTerminal {
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
        self.account_picker_open = false;
        self.account_picker_rename_index = None;

        if self.pending_order_action.is_some() {
            self.push_toast(
                "Wait for the pending order request before deleting an account".to_string(),
                true,
            );
            return Task::none();
        }

        let Some(profile_snapshot) = self.accounts.get(index).cloned() else {
            return Task::none();
        };

        if self.account_change_blocked_by_active_chase("deleting an account") {
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

        let (status_message, status_is_error) =
            match config::clear_profile_secrets(&profile_snapshot) {
                Ok(()) => (format!("Deleted account: {account_label}"), false),
                Err(e) => (
                    format!("Deleted '{account_label}', but keychain cleanup reported: {e}"),
                    true,
                ),
            };
        self.push_toast(status_message, status_is_error);

        if self.secret_storage_mode == config::CredentialStorageMode::EncryptedConfig
            && !self.persist_encrypted_credentials_blob("Deleted account from encrypted config")
        {
            self.push_toast(
                format!(
                    "Deleted '{account_label}' in memory, but encrypted credential cleanup failed; config was not saved"
                ),
                true,
            );
            return Task::none();
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
