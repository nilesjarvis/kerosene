use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Secret Persistence
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn committed_config_save_warning(action: &str, error: &str) -> String {
        format!(
            "{action}, but config durability could not be fully verified: {}",
            redact_sensitive_response_text(error)
        )
    }

    fn persist_keychain_secret_update(
        &mut self,
        update: config::KeychainSecretUpdate<'_>,
        success_message: &str,
        failure_prefix: &str,
    ) -> bool {
        match config::update_keychain_secret_payload(update) {
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
        let openrouter_api_key = Zeroizing::new(self.openrouter_api_key.as_str().to_string());
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
            openrouter_api_key.as_str(),
        )
    }

    pub(crate) fn persist_profile_agent_key_removal_from_accounts(
        &mut self,
        accounts: &[config::AccountProfile],
        removed_profile_secret_id: &str,
    ) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_secret_update(
                config::KeychainSecretUpdate::RemoveProfile(removed_profile_secret_id),
                "Agent key removed from OS keychain",
                "Keychain update failed; wallet address was not changed",
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

        let Some(secret_id) = self
            .accounts
            .get(self.active_account_index)
            .map(|profile| profile.secret_id.clone())
        else {
            self.secret_store_status = Some(("No active account to save".into(), true));
            return false;
        };
        self.persist_profile_secrets_from_accounts(accounts, &secret_id)
    }

    pub(crate) fn persist_profile_secrets_from_accounts(
        &mut self,
        accounts: &[config::AccountProfile],
        profile_secret_id: &str,
    ) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                let Some(profile) = accounts
                    .iter()
                    .find(|profile| profile.secret_id == profile_secret_id)
                    .cloned()
                else {
                    self.secret_store_status =
                        Some(("Account credential save target was not found".into(), true));
                    return false;
                };
                self.persist_keychain_secret_update(
                    config::KeychainSecretUpdate::Profile(&profile),
                    "Credentials saved to OS keychain",
                    "Keychain save failed; credentials were not committed",
                )
            }
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
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_secret_update(
                config::KeychainSecretUpdate::Hydromancer(hydromancer_api_key),
                "Hydromancer key saved to OS keychain",
                "Hydromancer keychain save failed; key was not committed",
            ),
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
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_secret_update(
                config::KeychainSecretUpdate::Hyperdash(hyperdash_api_key),
                "HyperDash key saved to OS keychain",
                "HyperDash keychain save failed; key was not committed",
            ),
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
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_secret_update(
                config::KeychainSecretUpdate::XOAuth {
                    access_token: x_access_token,
                    client_id: x_oauth_client_id,
                    refresh_token: x_refresh_token,
                },
                "X credentials saved to OS keychain",
                "X credential keychain save failed; credentials were not committed",
            ),
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
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_secret_update(
                config::KeychainSecretUpdate::SchwabOAuth {
                    client_id: schwab_client_id,
                    client_secret: schwab_client_secret,
                    access_token: schwab_access_token,
                    refresh_token: schwab_refresh_token,
                },
                "Schwab credentials saved to OS keychain",
                "Schwab credential keychain save failed; credentials were not committed",
            ),
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
                    &self.openrouter_api_key,
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

    pub(crate) fn persist_openrouter_secret_from_key(&mut self, openrouter_api_key: &str) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_secret_update(
                config::KeychainSecretUpdate::OpenRouter(openrouter_api_key),
                "OpenRouter key saved to OS keychain",
                "OpenRouter keychain save failed; key was not committed",
            ),
            config::CredentialStorageMode::EncryptedConfig => {
                let accounts = self.persisted_accounts_snapshot();
                let (x_access_token, x_oauth_client_id, x_refresh_token) =
                    self.x_feed.oauth_credentials_for_secret();
                let (
                    schwab_client_id,
                    schwab_client_secret,
                    schwab_access_token,
                    schwab_refresh_token,
                ) = self.schwab.oauth_credentials_for_secret();
                let payload = config::SecretPayload::from_credentials_with_integrations(
                    &accounts,
                    &self.hydromancer_api_key,
                    &self.hyperdash_api_key,
                    x_access_token.as_str(),
                    x_oauth_client_id.as_str(),
                    x_refresh_token.as_str(),
                    schwab_client_id.as_str(),
                    schwab_client_secret.as_str(),
                    schwab_access_token.as_str(),
                    schwab_refresh_token.as_str(),
                    openrouter_api_key,
                );
                let persisted = self.persist_encrypted_secret_payload(
                    payload,
                    "OpenRouter key saved to encrypted config",
                );
                self.secret_migration_save_blocked = !persisted;
                persisted
            }
        }
    }
}
