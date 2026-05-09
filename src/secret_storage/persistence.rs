use crate::app_state::TradingTerminal;
use crate::config;

// ---------------------------------------------------------------------------
// Secret Persistence
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn persist_active_profile_secrets(&mut self) -> bool {
        if self.active_account_is_ghost() {
            self.secret_store_status = Some(("Ghost wallets are in memory only".into(), false));
            return true;
        }

        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                let Some(profile) = self.accounts.get(self.active_account_index) else {
                    return true;
                };

                match config::store_profile_secrets(profile) {
                    Ok(()) => {
                        self.secret_store_status =
                            Some(("Credentials saved to OS keychain".into(), false));
                        true
                    }
                    Err(error) => {
                        self.secret_store_status = Some((
                            format!(
                                "Keychain save failed; credentials are only in memory: {error}"
                            ),
                            true,
                        ));
                        false
                    }
                }
            }
            config::CredentialStorageMode::EncryptedConfig => {
                self.persist_encrypted_credentials_blob("Credentials saved to encrypted config")
            }
        }
    }

    pub(crate) fn persist_hydromancer_secret(&mut self) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                match config::store_global_hydromancer_secret(&self.hydromancer_api_key) {
                    Ok(()) => {
                        self.secret_store_status =
                            Some(("Hydromancer key saved to OS keychain".into(), false));
                        true
                    }
                    Err(error) => {
                        self.secret_store_status = Some((
                            format!(
                                "Hydromancer keychain save failed; key is only in memory: {error}"
                            ),
                            true,
                        ));
                        false
                    }
                }
            }
            config::CredentialStorageMode::EncryptedConfig => {
                self.persist_encrypted_credentials_blob("Hydromancer key saved to encrypted config")
            }
        }
    }

    pub(crate) fn persist_hyperdash_secret(&mut self) -> bool {
        match self.secret_storage_mode {
            config::CredentialStorageMode::OsKeychain => {
                match config::store_global_hyperdash_secret(&self.hyperdash_api_key) {
                    Ok(()) => {
                        self.secret_store_status =
                            Some(("HyperDash key saved to OS keychain".into(), false));
                        true
                    }
                    Err(error) => {
                        self.secret_store_status = Some((
                            format!(
                                "HyperDash keychain save failed; key is only in memory: {error}"
                            ),
                            true,
                        ));
                        false
                    }
                }
            }
            config::CredentialStorageMode::EncryptedConfig => {
                self.persist_encrypted_credentials_blob("HyperDash key saved to encrypted config")
            }
        }
    }
}
