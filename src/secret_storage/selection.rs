use crate::app_state::TradingTerminal;
use crate::config;
use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Secret Storage Selection
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn clear_keychain_credentials_best_effort(&mut self) {
        let mut errors = Vec::new();
        for profile in self.persisted_accounts_snapshot() {
            if let Err(error) = config::clear_profile_secrets(&profile) {
                errors.push(format!("{}: {error}", profile.name));
            }
        }
        if let Err(error) = config::clear_global_secrets() {
            errors.push(format!("global: {error}"));
        }
        if !errors.is_empty() {
            self.secret_store_status = Some((
                format!(
                    "Encrypted credentials saved; OS keychain cleanup skipped: {}",
                    errors.join("; ")
                ),
                false,
            ));
        }
    }

    pub(crate) fn apply_secret_storage_selection(&mut self) {
        match self.secret_storage_selection {
            config::CredentialStorageMode::OsKeychain => {
                if self.secret_storage_mode == config::CredentialStorageMode::EncryptedConfig
                    && !self.encrypted_secrets_unlocked
                {
                    self.secret_store_status = Some((
                        "Unlock encrypted credentials before moving them to the OS keychain"
                            .to_string(),
                        true,
                    ));
                    return;
                }

                let accounts = self.persisted_accounts_snapshot();
                match config::store_keychain_secrets(
                    &accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                ) {
                    Ok(()) => {
                        self.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
                        self.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
                        self.encrypted_secrets = None;
                        self.encrypted_secret_password.zeroize();
                        self.encrypted_secret_confirm.zeroize();
                        self.encrypted_secrets_unlocked = false;
                        self.secret_store_status =
                            Some(("Credentials saved to OS keychain".to_string(), false));
                        self.persist_config();
                    }
                    Err(error) => {
                        self.secret_storage_selection = self.secret_storage_mode;
                        self.secret_store_status =
                            Some((format!("OS keychain credential save failed: {error}"), true));
                    }
                }
            }
            config::CredentialStorageMode::EncryptedConfig => {
                if !self.encrypted_password_is_ready() {
                    return;
                }
                let confirm_required = self.secret_storage_mode
                    != config::CredentialStorageMode::EncryptedConfig
                    || self.encrypted_secrets.is_none()
                    || !self.encrypted_secret_confirm.is_empty();
                if confirm_required
                    && self.encrypted_secret_password != self.encrypted_secret_confirm
                {
                    self.secret_store_status = Some((
                        "Encrypted credential passwords do not match".to_string(),
                        true,
                    ));
                    return;
                }

                if self.persist_encrypted_credentials_blob("Credentials saved to encrypted config")
                {
                    self.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
                    self.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
                    self.encrypted_secret_confirm.zeroize();
                    self.persist_config();
                    self.clear_keychain_credentials_best_effort();
                }
            }
        }
    }
}
