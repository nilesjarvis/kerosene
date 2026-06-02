use crate::app_state::TradingTerminal;
use crate::config;
use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Encrypted Credentials
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn current_secret_payload(&self) -> config::SecretPayload {
        let accounts = self.persisted_accounts_snapshot();
        config::SecretPayload::from_credentials(
            &accounts,
            &self.hydromancer_api_key,
            &self.hyperdash_api_key,
            &self.x_feed.bearer_token,
        )
    }

    pub(crate) fn encrypted_password_is_ready(&mut self) -> bool {
        if self.encrypted_secret_password.trim().is_empty() {
            self.secret_store_status = Some((
                "Enter a password for encrypted credential storage".to_string(),
                true,
            ));
            return false;
        }
        true
    }

    pub(crate) fn encrypted_credentials_locked_for(
        mode: config::CredentialStorageMode,
        has_encrypted_secrets: bool,
        unlocked: bool,
    ) -> bool {
        mode == config::CredentialStorageMode::EncryptedConfig && has_encrypted_secrets && !unlocked
    }

    pub(crate) fn encrypted_credentials_locked(&self) -> bool {
        Self::encrypted_credentials_locked_for(
            self.secret_storage_mode,
            self.encrypted_secrets.is_some(),
            self.encrypted_secrets_unlocked,
        )
    }

    pub(crate) fn persist_encrypted_credentials_blob(&mut self, success_message: &str) -> bool {
        if self.encrypted_secrets.is_some() && !self.encrypted_secrets_unlocked {
            self.secret_store_status = Some((
                "Unlock encrypted credentials before saving changes".to_string(),
                true,
            ));
            return false;
        }
        if !self.encrypted_password_is_ready() {
            return false;
        }

        let payload = self.current_secret_payload();
        match config::encrypt_secrets(&payload, &self.encrypted_secret_password) {
            Ok(encrypted) => {
                self.encrypted_secrets = Some(encrypted);
                self.encrypted_secrets_unlocked = true;
                self.secret_store_status = Some((success_message.to_string(), false));
                true
            }
            Err(error) => {
                self.secret_store_status =
                    Some((format!("Encrypted credential save failed: {error}"), true));
                false
            }
        }
    }

    pub(crate) fn apply_secret_payload(&mut self, payload: config::SecretPayload) {
        for profile in &mut self.accounts {
            profile.agent_key.zeroize();
            if let Some(saved) = payload
                .profiles
                .iter()
                .find(|saved| saved.secret_id == profile.secret_id)
            {
                profile.agent_key = saved.agent_key.clone();
            }
        }

        self.wallet_key_input.zeroize();
        self.wallet_key_input = self
            .accounts
            .get(self.active_account_index)
            .map(|profile| profile.agent_key.clone())
            .unwrap_or_default();
        self.hydromancer_api_key.zeroize();
        self.hydromancer_api_key = payload.global.hydromancer_api_key;
        self.hydromancer_key_input.zeroize();
        self.hydromancer_key_input = self.hydromancer_api_key.clone();
        self.hyperdash_api_key.zeroize();
        self.hyperdash_api_key = payload.global.hyperdash_api_key;
        self.hyperdash_key_input.zeroize();
        self.hyperdash_key_input = self.hyperdash_api_key.clone();
        self.x_feed.bearer_token.zeroize();
        self.x_feed.bearer_token = payload.global.x_bearer_token;
        self.x_feed.bearer_token_input.zeroize();
        self.x_feed.bearer_token_input = self.x_feed.bearer_token.clone();
        self.x_feed.stream_connected = false;
        self.x_feed.stream_reconnect_nonce = self.x_feed.stream_reconnect_nonce.saturating_add(1);

        self.liquidations_last_rx_ms = None;
        self.tracked_trades_last_rx_ms = None;
        self.liquidations_reconnect_nonce = self.liquidations_reconnect_nonce.wrapping_add(1);
        self.tracked_trades_reconnect_nonce = self.tracked_trades_reconnect_nonce.wrapping_add(1);
        self.liquidations_status = if self.hydromancer_api_key.trim().is_empty() {
            "Disconnected".to_string()
        } else {
            "Connecting...".to_string()
        };
        self.tracked_trades_status = self.liquidations_status.clone();

        if !self.hydromancer_api_key.trim().is_empty() {
            crate::ws::reconnect_hydromancer(self.hydromancer_api_key.trim());
        }
    }

    pub(crate) fn unlock_encrypted_credentials(&mut self) {
        if !self.encrypted_password_is_ready() {
            return;
        }

        let Some(encrypted) = &self.encrypted_secrets else {
            self.secret_store_status = Some((
                "No encrypted credentials have been saved yet".to_string(),
                true,
            ));
            return;
        };

        match config::decrypt_secrets(encrypted, &self.encrypted_secret_password) {
            Ok(payload) => {
                self.apply_secret_payload(payload);
                self.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
                self.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
                self.encrypted_secrets_unlocked = true;
                self.show_unlock_credentials_popup = false;
                self.secret_store_status = Some(("Encrypted credentials unlocked".into(), false));
            }
            Err(error) => {
                self.encrypted_secrets_unlocked = false;
                self.secret_store_status =
                    Some((format!("Encrypted credential unlock failed: {error}"), true));
            }
        }
    }
}
