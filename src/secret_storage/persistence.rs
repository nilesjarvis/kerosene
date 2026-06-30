use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Secret Persistence
// ---------------------------------------------------------------------------

struct SecretPersistenceValues<'a> {
    accounts: &'a [config::AccountProfile],
    hydromancer_api_key: &'a str,
    hyperdash_api_key: &'a str,
    x_access_token: &'a str,
    removed_profile_secret_ids: &'a [String],
}

impl TradingTerminal {
    pub(crate) fn committed_config_save_warning(action: &str, error: &str) -> String {
        format!(
            "{action}, but config durability could not be fully verified: {}",
            redact_sensitive_response_text(error)
        )
    }

    fn persist_keychain_credentials_from_values(
        &mut self,
        values: SecretPersistenceValues<'_>,
        success_message: &str,
        failure_prefix: &str,
    ) -> bool {
        match config::store_keychain_secrets_with_profile_removals_with_x(
            values.accounts,
            values.hydromancer_api_key,
            values.hyperdash_api_key,
            values.x_access_token,
            values.removed_profile_secret_ids,
        ) {
            Ok(cleanup_warning) => {
                self.secret_migration_save_blocked = false;
                if let Some(cleanup_warning) = cleanup_warning {
                    self.secret_store_status = Some((
                        format!(
                            "{success_message}; legacy OS keychain cleanup skipped: {}",
                            redact_sensitive_response_text(&cleanup_warning)
                        ),
                        true,
                    ));
                } else {
                    self.secret_store_status = Some((success_message.into(), false));
                }
                true
            }
            Err(error) => {
                self.secret_migration_save_blocked = true;
                self.secret_store_status = Some((
                    format!(
                        "{failure_prefix}: {}. If OS keychain storage keeps failing, switch to encrypted config in Settings > Storage.",
                        redact_sensitive_response_text(&error)
                    ),
                    true,
                ));
                false
            }
        }
    }

    fn persist_keychain_credentials_from_accounts(
        &mut self,
        accounts: &[config::AccountProfile],
        success_message: &str,
        failure_prefix: &str,
        removed_profile_secret_ids: &[String],
    ) -> bool {
        let hydromancer_api_key = Zeroizing::new(self.hydromancer_api_key.as_str().to_string());
        let hyperdash_api_key = Zeroizing::new(self.hyperdash_api_key.as_str().to_string());
        let x_access_token = self.x_feed.access_token_for_secret();
        self.persist_keychain_credentials_from_values(
            SecretPersistenceValues {
                accounts,
                hydromancer_api_key: hydromancer_api_key.as_str(),
                hyperdash_api_key: hyperdash_api_key.as_str(),
                x_access_token: x_access_token.as_str(),
                removed_profile_secret_ids,
            },
            success_message,
            failure_prefix,
        )
    }

    pub(crate) fn persist_profile_agent_key_removal_from_accounts(
        &mut self,
        accounts: &[config::AccountProfile],
        removed_profile_secret_id: &str,
    ) -> bool {
        let removed_profile_secret_ids = [removed_profile_secret_id.to_string()];
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => self
                .persist_keychain_credentials_from_accounts(
                    accounts,
                    "Agent key removed from OS keychain",
                    "Keychain update failed; wallet address was not changed",
                    &removed_profile_secret_ids,
                ),
            config::CredentialStorageMode::EncryptedConfig => {
                let x_access_token = self.x_feed.access_token_for_secret();
                let payload = config::SecretPayload::from_credentials_with_x(
                    accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token.as_str(),
                );
                let persisted = self.persist_encrypted_secret_payload(
                    payload,
                    "Agent key removed from encrypted config",
                );
                self.secret_migration_save_blocked = !persisted;
                persisted
            }
        }
    }

    pub(crate) fn persist_active_profile_secrets(&mut self) -> bool {
        let accounts = self.persisted_accounts_snapshot();
        self.persist_active_profile_secrets_from_accounts(&accounts)
    }

    pub(crate) fn persist_active_profile_secrets_from_accounts(
        &mut self,
        accounts: &[config::AccountProfile],
    ) -> bool {
        if self.active_account_is_ghost() {
            self.secret_store_status = Some(("Ghost wallets are in memory only".into(), false));
            return true;
        }

        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => self
                .persist_keychain_credentials_from_accounts(
                    accounts,
                    "Credentials saved to OS keychain",
                    "Keychain save failed; credentials were not committed",
                    &[],
                ),
            config::CredentialStorageMode::EncryptedConfig => {
                let x_access_token = self.x_feed.access_token_for_secret();
                let payload = config::SecretPayload::from_credentials_with_x(
                    accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token.as_str(),
                );
                let persisted = self.persist_encrypted_secret_payload(
                    payload,
                    "Credentials saved to encrypted config",
                );
                self.secret_migration_save_blocked = !persisted;
                persisted
            }
        }
    }

    pub(crate) fn persist_hydromancer_secret_from_key(
        &mut self,
        hydromancer_api_key: &str,
    ) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                let accounts = self.persisted_accounts_snapshot();
                let hyperdash_api_key = Zeroizing::new(self.hyperdash_api_key.as_str().to_string());
                let x_access_token = self.x_feed.access_token_for_secret();
                self.persist_keychain_credentials_from_values(
                    SecretPersistenceValues {
                        accounts: &accounts,
                        hydromancer_api_key,
                        hyperdash_api_key: hyperdash_api_key.as_str(),
                        x_access_token: x_access_token.as_str(),
                        removed_profile_secret_ids: &[],
                    },
                    "Hydromancer key saved to OS keychain",
                    "Hydromancer keychain save failed; key was not committed",
                )
            }
            config::CredentialStorageMode::EncryptedConfig => {
                let accounts = self.persisted_accounts_snapshot();
                let x_access_token = self.x_feed.access_token_for_secret();
                let payload = config::SecretPayload::from_credentials_with_x(
                    &accounts,
                    hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token.as_str(),
                );
                let persisted = self.persist_encrypted_secret_payload(
                    payload,
                    "Hydromancer key saved to encrypted config",
                );
                self.secret_migration_save_blocked = !persisted;
                persisted
            }
        }
    }

    pub(crate) fn persist_hyperdash_secret_from_key(&mut self, hyperdash_api_key: &str) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                let accounts = self.persisted_accounts_snapshot();
                let hydromancer_api_key =
                    Zeroizing::new(self.hydromancer_api_key.as_str().to_string());
                let x_access_token = self.x_feed.access_token_for_secret();
                self.persist_keychain_credentials_from_values(
                    SecretPersistenceValues {
                        accounts: &accounts,
                        hydromancer_api_key: hydromancer_api_key.as_str(),
                        hyperdash_api_key,
                        x_access_token: x_access_token.as_str(),
                        removed_profile_secret_ids: &[],
                    },
                    "HyperDash key saved to OS keychain",
                    "HyperDash keychain save failed; key was not committed",
                )
            }
            config::CredentialStorageMode::EncryptedConfig => {
                let accounts = self.persisted_accounts_snapshot();
                let x_access_token = self.x_feed.access_token_for_secret();
                let payload = config::SecretPayload::from_credentials_with_x(
                    &accounts,
                    &self.hydromancer_api_key,
                    hyperdash_api_key,
                    x_access_token.as_str(),
                );
                let persisted = self.persist_encrypted_secret_payload(
                    payload,
                    "HyperDash key saved to encrypted config",
                );
                self.secret_migration_save_blocked = !persisted;
                persisted
            }
        }
    }

    pub(crate) fn persist_x_access_token_secret_from_key(&mut self, x_access_token: &str) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                let accounts = self.persisted_accounts_snapshot();
                let hydromancer_api_key =
                    Zeroizing::new(self.hydromancer_api_key.as_str().to_string());
                let hyperdash_api_key = Zeroizing::new(self.hyperdash_api_key.as_str().to_string());
                self.persist_keychain_credentials_from_values(
                    SecretPersistenceValues {
                        accounts: &accounts,
                        hydromancer_api_key: hydromancer_api_key.as_str(),
                        hyperdash_api_key: hyperdash_api_key.as_str(),
                        x_access_token,
                        removed_profile_secret_ids: &[],
                    },
                    "X access token saved to OS keychain",
                    "X access token keychain save failed; token was not committed",
                )
            }
            config::CredentialStorageMode::EncryptedConfig => {
                let accounts = self.persisted_accounts_snapshot();
                let payload = config::SecretPayload::from_credentials_with_x(
                    &accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token,
                );
                let persisted = self.persist_encrypted_secret_payload(
                    payload,
                    "X access token saved to encrypted config",
                );
                self.secret_migration_save_blocked = !persisted;
                persisted
            }
        }
    }
}
