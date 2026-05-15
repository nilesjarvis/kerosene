use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::config;
use crate::message::Message;
use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn update_hyperdash_key(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HyperdashKeyInputChanged(value) => {
                self.hyperdash_key_input.zeroize();
                self.hyperdash_key_input = value.into();
            }
            Message::SaveHyperdashKey => {
                let previous_key = self.hyperdash_api_key.trim().to_string();
                let new_key = self.hyperdash_key_input.trim().to_string();
                self.hyperdash_api_key.zeroize();
                self.hyperdash_api_key = new_key.into();
                if self.secret_storage_mode == config::CredentialStorageMode::EncryptedConfig
                    && !self.persist_hyperdash_secret()
                {
                    self.hyperdash_api_key.zeroize();
                    self.hyperdash_api_key = previous_key.into();
                    return Task::none();
                }
                if self.secret_storage_mode != config::CredentialStorageMode::EncryptedConfig {
                    self.persist_hyperdash_secret();
                }
                self.persist_config();
                let heatmap_ids: Vec<ChartId> = self
                    .charts
                    .iter()
                    .filter(|(_, inst)| inst.show_heatmap && !inst.symbol.is_empty())
                    .map(|(id, _)| *id)
                    .collect();
                let liquidation_ids: Vec<ChartId> = self
                    .charts
                    .iter()
                    .filter(|(_, inst)| inst.show_liquidations && !inst.symbol.is_empty())
                    .map(|(id, _)| *id)
                    .collect();
                self.liquidation_pending_charts.clear();
                for id in &liquidation_ids {
                    if let Some(instance) = self.charts.get_mut(id) {
                        instance.liquidation_fetching = false;
                        instance.liquidation_pending_key = None;
                    }
                }
                if self.hyperdash_api_key.is_empty() {
                    self.heatmap_pending_charts.clear();
                    for id in heatmap_ids {
                        if let Some(instance) = self.charts.get_mut(&id) {
                            instance.heatmap_fetching = false;
                            instance.heatmap_last_fetch = None;
                            instance.heatmap_status = Some((
                                "Add HyperDash key in Settings > Integrations".to_string(),
                                true,
                            ));
                            Self::clear_heatmap_display(instance);
                        }
                    }
                    for id in liquidation_ids {
                        if let Some(instance) = self.charts.get_mut(&id) {
                            Self::clear_liquidation_display(instance);
                            instance.liquidation_status = Some((
                                "Add HyperDash key in Settings > Integrations".to_string(),
                                true,
                            ));
                            instance.chart.candle_cache.clear();
                        }
                    }
                    return Task::none();
                }
                let mut tasks: Vec<Task<Message>> = heatmap_ids
                    .into_iter()
                    .map(|id| self.maybe_fetch_heatmap(id))
                    .collect();
                tasks.extend(
                    liquidation_ids
                        .into_iter()
                        .map(|id| self.maybe_fetch_liquidations(id)),
                );
                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
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
    fn save_hyperdash_key_locked_encrypted_credentials_does_not_activate_key() {
        let mut terminal = locked_encrypted_credentials_terminal();
        terminal.hyperdash_api_key = "old-hyper".to_string().into();
        terminal.hyperdash_key_input = "new-hyper".to_string().into();

        let _ = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

        assert_eq!(terminal.hyperdash_api_key.trim(), "old-hyper");
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
    fn save_hyperdash_key_missing_encrypted_password_does_not_activate_key() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secrets = None;
        terminal.encrypted_secrets_unlocked = false;
        terminal.encrypted_secret_password = String::new().into();
        terminal.hyperdash_api_key = "old-hyper".to_string().into();
        terminal.hyperdash_key_input = "new-hyper".to_string().into();

        let _ = terminal.update_hyperdash_key(Message::SaveHyperdashKey);

        assert_eq!(terminal.hyperdash_api_key.trim(), "old-hyper");
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
