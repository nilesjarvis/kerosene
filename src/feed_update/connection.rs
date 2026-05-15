use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use crate::ws;
use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn update_feed_connection(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HydromancerKeyInputChanged(value) => {
                self.hydromancer_key_input.zeroize();
                self.hydromancer_key_input = value.into();
            }
            Message::SaveHydromancerKey => {
                // Capture the previous trimmed key BEFORE zeroizing the
                // in-memory state so we can evict its process-wide manager.
                // Without this, the spawned task keeps its owned `api_key`
                // String alive for the lifetime of the app even though
                // the user explicitly rotated/cleared the credential.
                let previous_key = self.hydromancer_api_key.trim().to_string();
                let new_key = self.hydromancer_key_input.trim().to_string();
                self.hydromancer_api_key.zeroize();
                self.hydromancer_api_key = new_key.into();
                if self.secret_storage_mode == config::CredentialStorageMode::EncryptedConfig
                    && !self.persist_hydromancer_secret()
                {
                    self.hydromancer_api_key.zeroize();
                    self.hydromancer_api_key = previous_key.into();
                    return Task::none();
                }
                if !previous_key.is_empty() && previous_key != self.hydromancer_api_key.trim() {
                    ws::evict_hydromancer_manager(&previous_key);
                }
                self.liquidations_last_rx_ms = None;
                self.liquidations_reconnect_nonce =
                    self.liquidations_reconnect_nonce.wrapping_add(1);
                self.tracked_trades_last_rx_ms = None;
                self.tracked_trades_reconnect_nonce =
                    self.tracked_trades_reconnect_nonce.wrapping_add(1);
                self.liquidations_status = if self.hydromancer_api_key.trim().is_empty() {
                    "Disconnected".to_string()
                } else {
                    "Connecting...".to_string()
                };
                self.tracked_trades_status = self.liquidations_status.clone();
                if self.secret_storage_mode != config::CredentialStorageMode::EncryptedConfig {
                    self.persist_hydromancer_secret();
                }
                self.persist_config();
                if !self.hydromancer_api_key.trim().is_empty() {
                    ws::reconnect_hydromancer(self.hydromancer_api_key.trim());
                }
                return self.refresh_enabled_funding_charts();
            }
            Message::ReconnectLiquidations if !self.hydromancer_api_key.trim().is_empty() => {
                ws::reconnect_hydromancer(self.hydromancer_api_key.trim());
                self.liquidations_last_rx_ms = None;
                self.liquidations_reconnect_nonce =
                    self.liquidations_reconnect_nonce.wrapping_add(1);
                self.liquidations_status = "Connecting...".to_string();
            }
            Message::ReconnectTrackedTrades if !self.hydromancer_api_key.trim().is_empty() => {
                ws::reconnect_hydromancer(self.hydromancer_api_key.trim());
                self.tracked_trades_last_rx_ms = None;
                self.tracked_trades_reconnect_nonce =
                    self.tracked_trades_reconnect_nonce.wrapping_add(1);
                self.tracked_trades_status = "Connecting...".to_string();
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locked_encrypted_credentials_terminal() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secrets = Some(
            config::encrypt_secrets(&terminal.current_secret_payload(), "test-password")
                .expect("test credentials should encrypt"),
        );
        terminal.encrypted_secrets_unlocked = false;
        terminal
    }

    #[test]
    fn save_hydromancer_key_locked_encrypted_credentials_does_not_activate_key() {
        let mut terminal = locked_encrypted_credentials_terminal();
        terminal.hydromancer_api_key = "old-hydro".to_string().into();
        terminal.hydromancer_key_input = "new-hydro".to_string().into();
        terminal.liquidations_reconnect_nonce = 7;
        terminal.tracked_trades_reconnect_nonce = 11;
        terminal.liquidations_status = "Connected".to_string();
        terminal.tracked_trades_status = "Streaming".to_string();

        let _ = terminal.update_feed_connection(Message::SaveHydromancerKey);

        assert_eq!(terminal.hydromancer_api_key.trim(), "old-hydro");
        assert_eq!(terminal.liquidations_reconnect_nonce, 7);
        assert_eq!(terminal.tracked_trades_reconnect_nonce, 11);
        assert_eq!(terminal.liquidations_status, "Connected");
        assert_eq!(terminal.tracked_trades_status, "Streaming");
        assert!(terminal.config_save_due_at.is_none());
        assert_eq!(
            terminal
                .secret_store_status
                .as_ref()
                .map(|(status, is_error)| (status.as_str(), *is_error)),
            Some(("Unlock encrypted credentials before saving changes", true))
        );
    }

    #[test]
    fn save_hydromancer_key_missing_encrypted_password_does_not_activate_key() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secrets = None;
        terminal.encrypted_secrets_unlocked = false;
        terminal.encrypted_secret_password = String::new().into();
        terminal.hydromancer_api_key = "old-hydro".to_string().into();
        terminal.hydromancer_key_input = "new-hydro".to_string().into();
        terminal.liquidations_reconnect_nonce = 3;
        terminal.tracked_trades_reconnect_nonce = 5;

        let _ = terminal.update_feed_connection(Message::SaveHydromancerKey);

        assert_eq!(terminal.hydromancer_api_key.trim(), "old-hydro");
        assert_eq!(terminal.liquidations_reconnect_nonce, 3);
        assert_eq!(terminal.tracked_trades_reconnect_nonce, 5);
        assert!(terminal.config_save_due_at.is_none());
        assert_eq!(
            terminal
                .secret_store_status
                .as_ref()
                .map(|(status, is_error)| (status.as_str(), *is_error)),
            Some(("Enter a password for encrypted credential storage", true))
        );
    }
}
