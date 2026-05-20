use crate::app_state::TradingTerminal;
use crate::config;

// ---------------------------------------------------------------------------
// Secret Persistence
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn persist_keychain_credentials(
        &mut self,
        success_message: &str,
        failure_prefix: &str,
    ) -> bool {
        let accounts = self.persisted_accounts_snapshot();
        match config::store_keychain_secrets(
            &accounts,
            &self.hydromancer_api_key,
            &self.hyperdash_api_key,
        ) {
            Ok(()) => {
                self.secret_store_status = Some((success_message.into(), false));
                true
            }
            Err(error) => {
                self.secret_store_status = Some((format!("{failure_prefix}: {error}"), true));
                false
            }
        }
    }

    pub(crate) fn persist_active_profile_secrets(&mut self) -> bool {
        if self.active_account_is_ghost() {
            self.secret_store_status = Some(("Ghost wallets are in memory only".into(), false));
            return true;
        }

        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_credentials(
                "Credentials saved to OS keychain",
                "Keychain save failed; credentials are only in memory",
            ),
            config::CredentialStorageMode::EncryptedConfig => {
                self.persist_encrypted_credentials_blob("Credentials saved to encrypted config")
            }
        }
    }

    pub(crate) fn persist_hydromancer_secret(&mut self) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_credentials(
                "Hydromancer key saved to OS keychain",
                "Hydromancer keychain save failed; key is only in memory",
            ),
            config::CredentialStorageMode::EncryptedConfig => {
                self.persist_encrypted_credentials_blob("Hydromancer key saved to encrypted config")
            }
        }
    }

    pub(crate) fn persist_hyperdash_secret(&mut self) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => self.persist_keychain_credentials(
                "HyperDash key saved to OS keychain",
                "HyperDash keychain save failed; key is only in memory",
            ),
            config::CredentialStorageMode::EncryptedConfig => {
                self.persist_encrypted_credentials_blob("HyperDash key saved to encrypted config")
            }
        }
    }
}
