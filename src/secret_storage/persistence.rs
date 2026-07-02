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
    x_oauth_client_id: &'a str,
    x_refresh_token: &'a str,
    schwab_client_id: &'a str,
    schwab_client_secret: &'a str,
    schwab_access_token: &'a str,
    schwab_refresh_token: &'a str,
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
        match config::store_keychain_secrets_with_profile_removals_with_integrations(
            values.accounts,
            values.hydromancer_api_key,
            values.hyperdash_api_key,
            values.x_access_token,
            values.x_oauth_client_id,
            values.x_refresh_token,
            values.schwab_client_id,
            values.schwab_client_secret,
            values.schwab_access_token,
            values.schwab_refresh_token,
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
        let (x_access_token, x_oauth_client_id, x_refresh_token) =
            self.x_feed.oauth_credentials_for_secret();
        let (schwab_client_id, schwab_client_secret, schwab_access_token, schwab_refresh_token) =
            self.schwab.oauth_credentials_for_secret();
        self.persist_keychain_credentials_from_values(
            SecretPersistenceValues {
                accounts,
                hydromancer_api_key: hydromancer_api_key.as_str(),
                hyperdash_api_key: hyperdash_api_key.as_str(),
                x_access_token: x_access_token.as_str(),
                x_oauth_client_id: x_oauth_client_id.as_str(),
                x_refresh_token: x_refresh_token.as_str(),
                schwab_client_id: schwab_client_id.as_str(),
                schwab_client_secret: schwab_client_secret.as_str(),
                schwab_access_token: schwab_access_token.as_str(),
                schwab_refresh_token: schwab_refresh_token.as_str(),
                removed_profile_secret_ids,
            },
            success_message,
            failure_prefix,
        )
    }

    pub(crate) fn secret_payload_with_current_integrations(
        &self,
        accounts: &[config::AccountProfile],
        hydromancer_api_key: &str,
        hyperdash_api_key: &str,
        x_access_token: &str,
        x_oauth_client_id: &str,
        x_refresh_token: &str,
    ) -> config::SecretPayload {
        let (schwab_client_id, schwab_client_secret, schwab_access_token, schwab_refresh_token) =
            self.schwab.oauth_credentials_for_secret();
        config::SecretPayload::from_credentials_with_integrations(
            accounts,
            hydromancer_api_key,
            hyperdash_api_key,
            x_access_token,
            x_oauth_client_id,
            x_refresh_token,
            schwab_client_id.as_str(),
            schwab_client_secret.as_str(),
            schwab_access_token.as_str(),
            schwab_refresh_token.as_str(),
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
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                let payload = self.secret_payload_with_current_integrations(
                    accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token.as_str(),
                    x_oauth_client_id.as_str(),
                    x_refresh_token.as_str(),
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

        self.persist_profile_secrets_from_accounts(accounts)
    }

    pub(crate) fn persist_profile_secrets_from_accounts(
        &mut self,
        accounts: &[config::AccountProfile],
    ) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => self
                .persist_keychain_credentials_from_accounts(
                    accounts,
                    "Credentials saved to OS keychain",
                    "Keychain save failed; credentials were not committed",
                    &[],
                ),
            config::CredentialStorageMode::EncryptedConfig => {
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                let payload = self.secret_payload_with_current_integrations(
                    accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token.as_str(),
                    x_oauth_client_id.as_str(),
                    x_refresh_token.as_str(),
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
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                let (
                    schwab_client_id,
                    schwab_client_secret,
                    schwab_access_token,
                    schwab_refresh_token,
                ) = self.schwab.oauth_credentials_for_secret();
                self.persist_keychain_credentials_from_values(
                    SecretPersistenceValues {
                        accounts: &accounts,
                        hydromancer_api_key,
                        hyperdash_api_key: hyperdash_api_key.as_str(),
                        x_access_token: x_access_token.as_str(),
                        x_oauth_client_id: x_oauth_client_id.as_str(),
                        x_refresh_token: x_refresh_token.as_str(),
                        schwab_client_id: schwab_client_id.as_str(),
                        schwab_client_secret: schwab_client_secret.as_str(),
                        schwab_access_token: schwab_access_token.as_str(),
                        schwab_refresh_token: schwab_refresh_token.as_str(),
                        removed_profile_secret_ids: &[],
                    },
                    "Hydromancer key saved to OS keychain",
                    "Hydromancer keychain save failed; key was not committed",
                )
            }
            config::CredentialStorageMode::EncryptedConfig => {
                let accounts = self.persisted_accounts_snapshot();
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                let payload = self.secret_payload_with_current_integrations(
                    &accounts,
                    hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token.as_str(),
                    x_oauth_client_id.as_str(),
                    x_refresh_token.as_str(),
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
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                let (
                    schwab_client_id,
                    schwab_client_secret,
                    schwab_access_token,
                    schwab_refresh_token,
                ) = self.schwab.oauth_credentials_for_secret();
                self.persist_keychain_credentials_from_values(
                    SecretPersistenceValues {
                        accounts: &accounts,
                        hydromancer_api_key: hydromancer_api_key.as_str(),
                        hyperdash_api_key,
                        x_access_token: x_access_token.as_str(),
                        x_oauth_client_id: x_oauth_client_id.as_str(),
                        x_refresh_token: x_refresh_token.as_str(),
                        schwab_client_id: schwab_client_id.as_str(),
                        schwab_client_secret: schwab_client_secret.as_str(),
                        schwab_access_token: schwab_access_token.as_str(),
                        schwab_refresh_token: schwab_refresh_token.as_str(),
                        removed_profile_secret_ids: &[],
                    },
                    "HyperDash key saved to OS keychain",
                    "HyperDash keychain save failed; key was not committed",
                )
            }
            config::CredentialStorageMode::EncryptedConfig => {
                let accounts = self.persisted_accounts_snapshot();
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                let payload = self.secret_payload_with_current_integrations(
                    &accounts,
                    &self.hydromancer_api_key,
                    hyperdash_api_key,
                    x_access_token.as_str(),
                    x_oauth_client_id.as_str(),
                    x_refresh_token.as_str(),
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

    pub(crate) fn persist_x_oauth_credentials_secret_from_keys(
        &mut self,
        x_access_token: &str,
        x_oauth_client_id: &str,
        x_refresh_token: &str,
    ) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                let accounts = self.persisted_accounts_snapshot();
                let hydromancer_api_key =
                    Zeroizing::new(self.hydromancer_api_key.as_str().to_string());
                let hyperdash_api_key = Zeroizing::new(self.hyperdash_api_key.as_str().to_string());
                let (
                    schwab_client_id,
                    schwab_client_secret,
                    schwab_access_token,
                    schwab_refresh_token,
                ) = self.schwab.oauth_credentials_for_secret();
                self.persist_keychain_credentials_from_values(
                    SecretPersistenceValues {
                        accounts: &accounts,
                        hydromancer_api_key: hydromancer_api_key.as_str(),
                        hyperdash_api_key: hyperdash_api_key.as_str(),
                        x_access_token,
                        x_oauth_client_id,
                        x_refresh_token,
                        schwab_client_id: schwab_client_id.as_str(),
                        schwab_client_secret: schwab_client_secret.as_str(),
                        schwab_access_token: schwab_access_token.as_str(),
                        schwab_refresh_token: schwab_refresh_token.as_str(),
                        removed_profile_secret_ids: &[],
                    },
                    "X credentials saved to OS keychain",
                    "X credential keychain save failed; credentials were not committed",
                )
            }
            config::CredentialStorageMode::EncryptedConfig => {
                let accounts = self.persisted_accounts_snapshot();
                let payload = self.secret_payload_with_current_integrations(
                    &accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token,
                    x_oauth_client_id,
                    x_refresh_token,
                );
                let persisted = self.persist_encrypted_secret_payload(
                    payload,
                    "X credentials saved to encrypted config",
                );
                self.secret_migration_save_blocked = !persisted;
                persisted
            }
        }
    }

    pub(crate) fn persist_schwab_credentials_secret_from_keys(
        &mut self,
        schwab_client_id: &str,
        schwab_client_secret: &str,
        schwab_access_token: &str,
        schwab_refresh_token: &str,
    ) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                let accounts = self.persisted_accounts_snapshot();
                let hydromancer_api_key =
                    Zeroizing::new(self.hydromancer_api_key.as_str().to_string());
                let hyperdash_api_key = Zeroizing::new(self.hyperdash_api_key.as_str().to_string());
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                self.persist_keychain_credentials_from_values(
                    SecretPersistenceValues {
                        accounts: &accounts,
                        hydromancer_api_key: hydromancer_api_key.as_str(),
                        hyperdash_api_key: hyperdash_api_key.as_str(),
                        x_access_token: x_access_token.as_str(),
                        x_oauth_client_id: x_oauth_client_id.as_str(),
                        x_refresh_token: x_refresh_token.as_str(),
                        schwab_client_id,
                        schwab_client_secret,
                        schwab_access_token,
                        schwab_refresh_token,
                        removed_profile_secret_ids: &[],
                    },
                    "Schwab credentials saved to OS keychain",
                    "Schwab credential keychain save failed; credentials were not committed",
                )
            }
            config::CredentialStorageMode::EncryptedConfig => {
                let accounts = self.persisted_accounts_snapshot();
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                let payload = config::SecretPayload::from_credentials_with_integrations(
                    &accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token.as_str(),
                    x_oauth_client_id.as_str(),
                    x_refresh_token.as_str(),
                    schwab_client_id,
                    schwab_client_secret,
                    schwab_access_token,
                    schwab_refresh_token,
                );
                let persisted = self.persist_encrypted_secret_payload(
                    payload,
                    "Schwab credentials saved to encrypted config",
                );
                self.secret_migration_save_blocked = !persisted;
                persisted
            }
        }
    }
}
