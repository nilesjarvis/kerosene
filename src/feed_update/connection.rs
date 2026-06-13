use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws;
use iced::Task;
use zeroize::{Zeroize, Zeroizing};

impl TradingTerminal {
    pub(crate) fn refresh_hydromancer_dependent_data(&mut self) -> Task<Message> {
        let mut tasks = vec![self.refresh_enabled_funding_charts()];
        if self.chart_backfill_source == crate::config::ChartBackfillSource::Hydromancer {
            tasks.push(self.reload_chart_backfills_for_source_change());
            tasks.push(self.refresh_account_data());
        }
        Task::batch(tasks)
    }

    pub(super) fn update_feed_connection(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HydromancerKeyInputChanged(value) => {
                self.hydromancer_key_input.zeroize();
                self.hydromancer_key_input = value.into_zeroizing().into();
            }
            Message::SaveHydromancerKey => {
                // Capture the previous trimmed key BEFORE zeroizing the
                // in-memory state so we can evict its process-wide manager.
                // Without this, the spawned task keeps its owned `api_key`
                // String alive for the lifetime of the app even though
                // the user explicitly rotated/cleared the credential.
                let previous_key = Zeroizing::new(self.hydromancer_api_key.trim().to_string());
                let previous_generation = self.hydromancer_key_generation;
                let next_key = Zeroizing::new(self.hydromancer_key_input.trim().to_string());
                if !self.persist_hydromancer_secret_from_key(next_key.as_str()) {
                    return Task::none();
                }

                self.hydromancer_api_key.zeroize();
                self.hydromancer_api_key = next_key.as_str().to_string().into();
                let hydromancer_key_changed = previous_key.as_str() != next_key.as_str();
                if !previous_key.is_empty() && hydromancer_key_changed {
                    ws::evict_hydromancer_manager(previous_generation);
                }
                if hydromancer_key_changed {
                    self.bump_hydromancer_key_generation();
                    self.journal.snapshot_requests.clear();
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
                }
                self.persist_config();
                if hydromancer_key_changed {
                    self.journal.clear_snapshot_cache();
                    self.journal.expanded_snapshot_trade_ids.clear();
                    if !self.hydromancer_api_key.trim().is_empty() {
                        ws::reconnect_hydromancer(self.hydromancer_key_generation);
                    }
                    return self.refresh_hydromancer_dependent_data();
                }
            }
            Message::ReconnectLiquidations if !self.hydromancer_api_key.trim().is_empty() => {
                self.liquidations_last_rx_ms = None;
                self.liquidations_reconnect_nonce =
                    self.liquidations_reconnect_nonce.wrapping_add(1);
                self.liquidations_status = "Connecting...".to_string();
            }
            Message::ReconnectTrackedTrades if !self.hydromancer_api_key.trim().is_empty() => {
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
    use crate::app_state::{TradingTerminal, sensitive_string};
    use crate::config::{self, ChartBackfillSource};
    use crate::journal::JournalTradeSnapshotRequest;
    use crate::timeframe::Timeframe;

    fn configure_encrypted_hydromancer_key(
        terminal: &mut TradingTerminal,
        hydromancer_key: &str,
        unlocked: bool,
    ) {
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secret_password = sensitive_string("test-password");
        terminal.encrypted_secrets = Some(
            config::encrypt_secrets(
                &config::SecretPayload::from_credentials(&[], hydromancer_key, "hyperdash-key"),
                &terminal.encrypted_secret_password,
            )
            .expect("test encrypted payload"),
        );
        terminal.encrypted_secrets_unlocked = unlocked;
        terminal.hyperdash_api_key = sensitive_string("hyperdash-key");
        terminal.secret_migration_save_blocked = false;
        terminal.secret_store_status = None;
    }

    fn snapshot_request(generation: u64) -> JournalTradeSnapshotRequest {
        JournalTradeSnapshotRequest {
            account_key: Some("acct".to_string()),
            address: "0xabc".to_string(),
            trade_id: "perp:BTC:test".to_string(),
            coin: "BTC".to_string(),
            source: ChartBackfillSource::Hydromancer,
            read_data_provider_generation: 0,
            hydromancer_key_generation: generation,
            timeframe: Timeframe::M1,
            ladder_index: 0,
            trade_start_ms: 1_000,
            trade_end_ms: 2_000,
            is_open: false,
            start_ms: 0,
            end_ms: 3_000,
        }
    }

    #[test]
    fn hydromancer_save_failure_preserves_live_key_generation_and_reconnect_state() {
        let mut terminal = TradingTerminal::boot().0;
        configure_encrypted_hydromancer_key(&mut terminal, "old-hydro", false);
        terminal.hydromancer_api_key = sensitive_string("old-hydro");
        terminal.hydromancer_key_input = sensitive_string("new-hydro");
        terminal.hydromancer_key_generation = 7;
        terminal.liquidations_last_rx_ms = Some(11);
        terminal.tracked_trades_last_rx_ms = Some(22);
        terminal.liquidations_reconnect_nonce = 3;
        terminal.tracked_trades_reconnect_nonce = 5;
        terminal.liquidations_status = "Liquidations current".to_string();
        terminal.tracked_trades_status = "Trades current".to_string();
        terminal.config_save_due_at = None;

        let _task = terminal.update_feed_connection(Message::SaveHydromancerKey);

        assert_eq!(terminal.hydromancer_api_key.as_str(), "old-hydro");
        assert_eq!(terminal.hydromancer_key_input.as_str(), "new-hydro");
        assert_eq!(terminal.hydromancer_key_generation, 7);
        assert_eq!(terminal.liquidations_last_rx_ms, Some(11));
        assert_eq!(terminal.tracked_trades_last_rx_ms, Some(22));
        assert_eq!(terminal.liquidations_reconnect_nonce, 3);
        assert_eq!(terminal.tracked_trades_reconnect_nonce, 5);
        assert_eq!(terminal.liquidations_status, "Liquidations current");
        assert_eq!(terminal.tracked_trades_status, "Trades current");
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Unlock encrypted credentials"));
    }

    #[test]
    fn hydromancer_clear_failure_keeps_committed_key_and_journal_snapshot_state() {
        let mut terminal = TradingTerminal::boot().0;
        configure_encrypted_hydromancer_key(&mut terminal, "old-hydro", false);
        terminal.hydromancer_api_key = sensitive_string("old-hydro");
        terminal.hydromancer_key_input = sensitive_string("");
        terminal.hydromancer_key_generation = 7;
        let request = snapshot_request(7);
        terminal
            .journal
            .snapshot_requests
            .insert(request.trade_id.clone(), request);
        terminal
            .journal
            .expanded_snapshot_trade_ids
            .insert("perp:BTC:test".to_string());
        terminal.config_save_due_at = None;

        let _task = terminal.update_feed_connection(Message::SaveHydromancerKey);

        assert_eq!(terminal.hydromancer_api_key.as_str(), "old-hydro");
        assert_eq!(terminal.hydromancer_key_generation, 7);
        assert!(
            terminal
                .journal
                .snapshot_requests
                .contains_key("perp:BTC:test")
        );
        assert!(
            terminal
                .journal
                .expanded_snapshot_trade_ids
                .contains("perp:BTC:test")
        );
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should remain present"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.global_hydromancer_api_key(), "old-hydro");
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn hydromancer_save_commits_after_encrypted_persistence_succeeds() {
        let mut terminal = TradingTerminal::boot().0;
        configure_encrypted_hydromancer_key(&mut terminal, "old-hydro", true);
        terminal.hydromancer_api_key = sensitive_string("old-hydro");
        terminal.hydromancer_key_input = sensitive_string("  new-hydro  ");
        terminal.hydromancer_key_generation = 7;
        terminal.liquidations_reconnect_nonce = 3;
        terminal.tracked_trades_reconnect_nonce = 5;
        let request = snapshot_request(7);
        terminal
            .journal
            .snapshot_requests
            .insert(request.trade_id.clone(), request);
        terminal
            .journal
            .expanded_snapshot_trade_ids
            .insert("perp:BTC:test".to_string());
        terminal.config_save_due_at = None;

        let _task = terminal.update_feed_connection(Message::SaveHydromancerKey);

        assert_eq!(terminal.hydromancer_api_key.as_str(), "new-hydro");
        assert_eq!(terminal.hydromancer_key_generation, 8);
        assert_eq!(terminal.liquidations_reconnect_nonce, 4);
        assert_eq!(terminal.tracked_trades_reconnect_nonce, 6);
        assert_eq!(terminal.liquidations_status, "Connecting...");
        assert_eq!(terminal.tracked_trades_status, "Connecting...");
        assert!(terminal.journal.snapshot_requests.is_empty());
        assert!(terminal.journal.expanded_snapshot_trade_ids.is_empty());
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should be rewritten"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.global_hydromancer_api_key(), "new-hydro");
        assert_eq!(payload.global_hyperdash_api_key(), "hyperdash-key");
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn hydromancer_same_key_save_persists_without_reconnecting_or_clearing_runtime_state() {
        let mut terminal = TradingTerminal::boot().0;
        configure_encrypted_hydromancer_key(&mut terminal, "hydro-secret", true);
        terminal.hydromancer_api_key = sensitive_string("hydro-secret");
        terminal.hydromancer_key_input = sensitive_string("  hydro-secret  ");
        terminal.hydromancer_key_generation = 991_000_003;
        terminal.liquidations_last_rx_ms = Some(11);
        terminal.tracked_trades_last_rx_ms = Some(22);
        terminal.liquidations_reconnect_nonce = 3;
        terminal.tracked_trades_reconnect_nonce = 5;
        terminal.liquidations_status = "Liquidations current".to_string();
        terminal.tracked_trades_status = "Trades current".to_string();
        let request = snapshot_request(terminal.hydromancer_key_generation);
        terminal
            .journal
            .snapshot_requests
            .insert(request.trade_id.clone(), request);
        terminal
            .journal
            .expanded_snapshot_trade_ids
            .insert("perp:BTC:test".to_string());
        terminal.config_save_due_at = None;

        let sent_manager_reconnect = ws::hydromancer_manager_reconnect_sent_for_test(
            terminal.hydromancer_key_generation,
            || {
                let _task = terminal.update_feed_connection(Message::SaveHydromancerKey);
            },
        );

        assert!(!sent_manager_reconnect);
        assert_eq!(terminal.hydromancer_api_key.as_str(), "hydro-secret");
        assert_eq!(terminal.hydromancer_key_generation, 991_000_003);
        assert_eq!(terminal.liquidations_last_rx_ms, Some(11));
        assert_eq!(terminal.tracked_trades_last_rx_ms, Some(22));
        assert_eq!(terminal.liquidations_reconnect_nonce, 3);
        assert_eq!(terminal.tracked_trades_reconnect_nonce, 5);
        assert_eq!(terminal.liquidations_status, "Liquidations current");
        assert_eq!(terminal.tracked_trades_status, "Trades current");
        assert!(
            terminal
                .journal
                .snapshot_requests
                .contains_key("perp:BTC:test")
        );
        assert!(
            terminal
                .journal
                .expanded_snapshot_trade_ids
                .contains("perp:BTC:test")
        );
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should remain saved"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.global_hydromancer_api_key(), "hydro-secret");
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_some());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(!*is_error);
        assert!(message.contains("Hydromancer key saved"));
    }

    #[test]
    fn reconnect_liquidations_restarts_only_liquidation_subscription_scope() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.hydromancer_api_key = sensitive_string("hydro-secret");
        terminal.hydromancer_key_generation = 991_000_001;
        terminal.liquidations_last_rx_ms = Some(11);
        terminal.tracked_trades_last_rx_ms = Some(22);
        terminal.liquidations_reconnect_nonce = 3;
        terminal.tracked_trades_reconnect_nonce = 5;
        terminal.liquidations_status = "Liquidations current".to_string();
        terminal.tracked_trades_status = "Trades current".to_string();

        let sent_manager_reconnect = ws::hydromancer_manager_reconnect_sent_for_test(
            terminal.hydromancer_key_generation,
            || {
                let _task = terminal.update_feed_connection(Message::ReconnectLiquidations);
            },
        );

        assert!(!sent_manager_reconnect);
        assert_eq!(terminal.liquidations_last_rx_ms, None);
        assert_eq!(terminal.liquidations_reconnect_nonce, 4);
        assert_eq!(terminal.liquidations_status, "Connecting...");
        assert_eq!(terminal.tracked_trades_last_rx_ms, Some(22));
        assert_eq!(terminal.tracked_trades_reconnect_nonce, 5);
        assert_eq!(terminal.tracked_trades_status, "Trades current");
    }

    #[test]
    fn reconnect_tracked_trades_restarts_only_tracked_trade_subscription_scope() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.hydromancer_api_key = sensitive_string("hydro-secret");
        terminal.hydromancer_key_generation = 991_000_002;
        terminal.liquidations_last_rx_ms = Some(11);
        terminal.tracked_trades_last_rx_ms = Some(22);
        terminal.liquidations_reconnect_nonce = 3;
        terminal.tracked_trades_reconnect_nonce = 5;
        terminal.liquidations_status = "Liquidations current".to_string();
        terminal.tracked_trades_status = "Trades current".to_string();

        let sent_manager_reconnect = ws::hydromancer_manager_reconnect_sent_for_test(
            terminal.hydromancer_key_generation,
            || {
                let _task = terminal.update_feed_connection(Message::ReconnectTrackedTrades);
            },
        );

        assert!(!sent_manager_reconnect);
        assert_eq!(terminal.liquidations_last_rx_ms, Some(11));
        assert_eq!(terminal.liquidations_reconnect_nonce, 3);
        assert_eq!(terminal.liquidations_status, "Liquidations current");
        assert_eq!(terminal.tracked_trades_last_rx_ms, None);
        assert_eq!(terminal.tracked_trades_reconnect_nonce, 6);
        assert_eq!(terminal.tracked_trades_status, "Connecting...");
    }
}
