use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::Task;
use zeroize::{Zeroize, Zeroizing};

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

    pub(crate) fn encrypted_secret_blob_for_payload(
        &mut self,
        payload: &config::SecretPayload,
    ) -> Option<config::EncryptedSecretsConfig> {
        if Self::encrypted_credentials_locked_for(
            self.secret_storage_mode,
            self.encrypted_secrets.is_some(),
            self.encrypted_secrets_unlocked,
        ) {
            self.secret_store_status = Some((
                "Unlock encrypted credentials before saving changes".to_string(),
                true,
            ));
            return None;
        }
        if !self.encrypted_password_is_ready() {
            return None;
        }

        match config::encrypt_secrets(payload, &self.encrypted_secret_password) {
            Ok(encrypted) => Some(encrypted),
            Err(error) => {
                self.secret_store_status =
                    Some((format!("Encrypted credential save failed: {error}"), true));
                None
            }
        }
    }

    pub(crate) fn persist_encrypted_secret_payload(
        &mut self,
        payload: config::SecretPayload,
        success_message: &str,
    ) -> bool {
        #[cfg(not(test))]
        {
            self.persist_encrypted_secret_payload_with(
                payload,
                success_message,
                config::save_config,
            )
        }
        #[cfg(test)]
        {
            self.persist_encrypted_secret_payload_with(payload, success_message, |_| Ok(()))
        }
    }

    pub(crate) fn persist_encrypted_secret_payload_with(
        &mut self,
        payload: config::SecretPayload,
        success_message: &str,
        save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
    ) -> bool {
        let Some(encrypted) = self.encrypted_secret_blob_for_payload(&payload) else {
            return false;
        };
        self.store_encrypted_secret_blob_immediately_with(encrypted, success_message, save_config)
    }

    pub(crate) fn store_encrypted_secret_blob_immediately_with(
        &mut self,
        encrypted: config::EncryptedSecretsConfig,
        success_message: &str,
        save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
    ) -> bool {
        let previous_encrypted_secrets = self.encrypted_secrets.clone();
        let previous_unlocked = self.encrypted_secrets_unlocked;

        self.encrypted_secrets = Some(encrypted);
        self.encrypted_secrets_unlocked = true;
        self.secret_migration_save_blocked = false;

        match self.persist_config_immediately_with(save_config) {
            Ok(()) => {
                self.secret_store_status = Some((success_message.to_string(), false));
                true
            }
            Err(error) => {
                if config::config_save_installed_snapshot(&error) {
                    self.secret_store_status = Some((
                        Self::committed_config_save_warning(success_message, &error),
                        true,
                    ));
                    return true;
                }
                self.encrypted_secrets = previous_encrypted_secrets;
                self.encrypted_secrets_unlocked = previous_unlocked;
                self.secret_migration_save_blocked = true;
                self.secret_store_status = Some((
                    format!(
                        "Encrypted credential config save failed: {error}; credential change was not committed"
                    ),
                    true,
                ));
                false
            }
        }
    }

    pub(crate) fn store_encrypted_secret_blob(
        &mut self,
        encrypted: config::EncryptedSecretsConfig,
        success_message: &str,
    ) {
        self.encrypted_secrets = Some(encrypted);
        self.encrypted_secrets_unlocked = true;
        self.secret_migration_save_blocked = false;
        self.secret_store_status = Some((success_message.to_string(), false));
    }

    pub(crate) fn apply_secret_payload(&mut self, payload: config::SecretPayload) -> usize {
        let mut skipped_bound_profile_keys = 0;
        for profile in &mut self.accounts {
            profile.agent_key.zeroize();
            if let Some(agent_key) =
                payload.profile_agent_key_for_wallet(&profile.secret_id, &profile.wallet_address)
            {
                profile.agent_key = agent_key.to_string().into();
            } else if payload
                .profile_agent_key_binding_mismatches(&profile.secret_id, &profile.wallet_address)
            {
                skipped_bound_profile_keys += 1;
            }
        }

        self.wallet_key_input.zeroize();
        self.wallet_key_input = self
            .accounts
            .get(self.active_account_index)
            .map(|profile| profile.agent_key.clone())
            .unwrap_or_default()
            .into();
        let previous_hydromancer_key = Zeroizing::new(self.hydromancer_api_key.trim().to_string());
        let previous_hydromancer_generation = self.hydromancer_key_generation;
        let hydromancer_key_changed =
            previous_hydromancer_key.as_str() != payload.global.hydromancer_api_key.trim();
        if !previous_hydromancer_key.is_empty() && hydromancer_key_changed {
            crate::ws::evict_hydromancer_manager(crate::ws::HydromancerStreamKey::from_zeroizing(
                previous_hydromancer_key.clone(),
                previous_hydromancer_generation,
            ));
        }
        self.hydromancer_api_key.zeroize();
        self.hydromancer_api_key = payload.global.hydromancer_api_key.into();
        self.hydromancer_key_input.zeroize();
        self.hydromancer_key_input = self.hydromancer_api_key.clone();
        if hydromancer_key_changed {
            self.bump_hydromancer_key_generation();
            self.journal.snapshot_requests.clear();
            self.journal.clear_snapshot_cache();
            self.journal.expanded_snapshot_trade_ids.clear();
        }
        let previous_hyperdash_key = Zeroizing::new(self.hyperdash_api_key.trim().to_string());
        let hyperdash_key_changed =
            previous_hyperdash_key.as_str() != payload.global.hyperdash_api_key.trim();
        self.hyperdash_api_key.zeroize();
        self.hyperdash_api_key = payload.global.hyperdash_api_key.into();
        self.hyperdash_key_input.zeroize();
        self.hyperdash_key_input = self.hyperdash_api_key.clone();
        if hyperdash_key_changed {
            self.bump_hyperdash_key_generation();
        }

        if hydromancer_key_changed {
            self.liquidations_last_rx_ms = None;
            self.tracked_trades_last_rx_ms = None;
            self.liquidations_reconnect_nonce = self.liquidations_reconnect_nonce.wrapping_add(1);
            self.tracked_trades_reconnect_nonce =
                self.tracked_trades_reconnect_nonce.wrapping_add(1);
            self.liquidations_status = if self.hydromancer_api_key.trim().is_empty() {
                "Disconnected".to_string()
            } else {
                "Connecting...".to_string()
            };
            self.tracked_trades_status = self.liquidations_status.clone();

            if !self.hydromancer_api_key.trim().is_empty() {
                crate::ws::reconnect_hydromancer(crate::ws::HydromancerStreamKey::from_zeroizing(
                    self.hydromancer_api_key_for_task(),
                    self.hydromancer_key_generation,
                ));
            }
        }

        skipped_bound_profile_keys
    }

    pub(crate) fn unlock_encrypted_credentials(&mut self) -> Task<Message> {
        self.unlock_encrypted_credentials_with_hooks(
            config::save_config,
            config::clear_all_keychain_secrets,
            config::clear_profile_secrets_by_id,
        )
    }

    #[cfg(test)]
    pub(crate) fn unlock_encrypted_credentials_with(
        &mut self,
        mut save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
    ) -> Task<Message> {
        self.unlock_encrypted_credentials_with_hooks(&mut save_config, |_| Ok(()), |_| Ok(()))
    }

    pub(crate) fn unlock_encrypted_credentials_with_hooks(
        &mut self,
        mut save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
        clear_all_keychain: impl FnOnce(&[config::AccountProfile]) -> Result<(), String>,
        clear_pending_profile: impl FnMut(&str) -> Result<(), String>,
    ) -> Task<Message> {
        if !self.encrypted_password_is_ready() {
            return Task::none();
        }

        let Some(encrypted) = &self.encrypted_secrets else {
            self.secret_store_status = Some((
                "No encrypted credentials have been saved yet".to_string(),
                true,
            ));
            return Task::none();
        };

        let hydromancer_generation_before = self.hydromancer_key_generation;
        match config::decrypt_secrets(encrypted, &self.encrypted_secret_password) {
            Ok(mut payload) => {
                let legacy_bindings_migrated = payload.bind_unbound_profile_agent_keys_to_wallets(
                    &self.persisted_accounts_snapshot(),
                );
                let skipped_bound_profile_keys = self.apply_secret_payload(payload.clone());
                self.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
                self.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
                self.encrypted_secrets_unlocked = true;
                self.show_unlock_credentials_popup = false;
                let migration_status = if legacy_bindings_migrated {
                    match config::encrypt_secrets(&payload, &self.encrypted_secret_password) {
                        Ok(encrypted) => {
                            let previous_encrypted_secrets = self.encrypted_secrets.clone();
                            self.encrypted_secrets = Some(encrypted);
                            self.secret_migration_save_blocked = false;
                            match self.persist_config_immediately_with(&mut save_config) {
                                Ok(()) => None,
                                Err(error) => {
                                    if config::config_save_installed_snapshot(&error) {
                                        Some(Self::committed_config_save_warning(
                                            "legacy wallet binding migration was saved",
                                            &error,
                                        ))
                                    } else {
                                        self.encrypted_secrets = previous_encrypted_secrets;
                                        self.secret_migration_save_blocked = true;
                                        Some(format!(
                                            "legacy wallet binding migration failed: config save failed: {error}; config saves are paused until credentials are saved to a working store"
                                        ))
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            self.secret_migration_save_blocked = true;
                            Some(format!(
                                "legacy wallet binding migration failed: {error}; config saves are paused until credentials are saved to a working store"
                            ))
                        }
                    }
                } else {
                    None
                };
                let cleanup_status = if migration_status.is_none() {
                    self.retry_pending_keychain_cleanup_after_encrypted_unlock_with(
                        &mut save_config,
                        clear_all_keychain,
                        clear_pending_profile,
                    )
                } else {
                    None
                };
                self.secret_store_status = if let Some(migration_status) = migration_status {
                    Some((
                        format!("Encrypted credentials unlocked; {migration_status}"),
                        true,
                    ))
                } else if let Some(cleanup_status) = cleanup_status {
                    Some((
                        format!("Encrypted credentials unlocked; {cleanup_status}"),
                        true,
                    ))
                } else if skipped_bound_profile_keys > 0 {
                    Some((
                        format!(
                            "Encrypted credentials unlocked; {skipped_bound_profile_keys} saved agent key(s) were skipped because their wallet binding does not match. Re-enter and save credentials for those accounts to trade."
                        ),
                        true,
                    ))
                } else {
                    Some(("Encrypted credentials unlocked".into(), false))
                };
                self.encrypted_secret_password.zeroize();
                self.encrypted_secret_confirm.zeroize();
                if self.hydromancer_key_generation != hydromancer_generation_before {
                    return self.refresh_hydromancer_dependent_data();
                }
                return Task::none();
            }
            Err(error) => {
                self.encrypted_secrets_unlocked = false;
                self.secret_store_status =
                    Some((format!("Encrypted credential unlock failed: {error}"), true));
            }
        }
        Task::none()
    }

    fn retry_pending_keychain_cleanup_after_encrypted_unlock_with(
        &mut self,
        save_config: &mut impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
        clear_all_keychain: impl FnOnce(&[config::AccountProfile]) -> Result<(), String>,
        mut clear_pending_profile: impl FnMut(&str) -> Result<(), String>,
    ) -> Option<String> {
        if self.pending_keychain_cleanup_all {
            let profiles = self.keychain_cleanup_profiles_snapshot();
            return match clear_all_keychain(&profiles) {
                Ok(()) => {
                    self.pending_keychain_cleanup_all = false;
                    self.pending_keychain_profile_deletions.clear();
                    self.persist_config_immediately_with(save_config)
                        .err()
                        .map(|error| {
                            format!(
                                "OS keychain cleanup completed after unlock, but cleanup state save failed: {error}"
                            )
                        })
                }
                Err(error) => {
                    let redacted = redact_keychain_cleanup_profile_ids(error, &profiles);
                    Some(format!(
                        "OS keychain cleanup failed and will retry after the next unlock: {redacted}"
                    ))
                }
            };
        }

        if self.pending_keychain_profile_deletions.is_empty() {
            return None;
        }

        let pending = std::mem::take(&mut self.pending_keychain_profile_deletions);
        let mut cleaned_any = false;
        let mut failures = Vec::new();
        for secret_id in pending {
            match clear_pending_profile(&secret_id) {
                Ok(()) => cleaned_any = true,
                Err(error) => {
                    let redacted = redact_keychain_cleanup_secret_id(error, &secret_id);
                    self.pending_keychain_profile_deletions.push(secret_id);
                    failures.push(redacted);
                }
            }
        }

        let save_error = if cleaned_any {
            self.persist_config_immediately_with(save_config).err()
        } else {
            None
        };

        match (failures.is_empty(), save_error) {
            (true, None) => None,
            (true, Some(error)) => Some(format!(
                "OS keychain account cleanup completed after unlock, but cleanup state save failed: {error}"
            )),
            (false, None) => Some(format!(
                "OS keychain account cleanup failed and will retry after the next unlock: {}",
                failures.join("; ")
            )),
            (false, Some(error)) => Some(format!(
                "OS keychain account cleanup partially failed and cleanup state save failed: {error}; cleanup will retry after the next unlock: {}",
                failures.join("; ")
            )),
        }
    }
}

fn redact_keychain_cleanup_profile_ids(
    error: String,
    profiles: &[config::AccountProfile],
) -> String {
    profiles.iter().fold(error, |message, profile| {
        redact_keychain_cleanup_secret_id(message, &profile.secret_id)
    })
}

fn redact_keychain_cleanup_secret_id(message: String, secret_id: &str) -> String {
    let trimmed = secret_id.trim();
    let mut redacted = message;
    if !secret_id.is_empty() {
        redacted = redacted.replace(secret_id, "<redacted-profile>");
    }
    if !trimmed.is_empty() && trimmed != secret_id {
        redacted = redacted.replace(trimmed, "<redacted-profile>");
    }
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::sensitive_string;
    use crate::chart_state::{CandleFetchRequest, ChartInstance};
    use crate::config::{AccountProfile, ChartBackfillSource, SecretPayload};
    use crate::journal::JournalTradeSnapshotRequest;
    use crate::timeframe::Timeframe;
    use crate::ws;
    use std::cell::{Cell, RefCell};

    fn account(secret_id: &str, wallet_address: &str) -> AccountProfile {
        AccountProfile {
            secret_id: secret_id.to_string(),
            name: secret_id.to_string(),
            wallet_address: wallet_address.to_string(),
            agent_key: sensitive_string("").into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }
    }

    fn terminal_with_encrypted_payload(payload: &SecretPayload, password: &str) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secret_password = sensitive_string(password);
        terminal.encrypted_secrets =
            Some(config::encrypt_secrets(payload, password).expect("encrypt fixture"));
        terminal.encrypted_secrets_unlocked = true;
        terminal.secret_migration_save_blocked = false;
        terminal.secret_store_status = None;
        terminal
    }

    fn hydromancer_stream_key(terminal: &TradingTerminal) -> ws::HydromancerStreamKey {
        ws::HydromancerStreamKey::from_zeroizing(
            terminal.hydromancer_api_key_for_task(),
            terminal.hydromancer_key_generation,
        )
    }

    #[test]
    fn encrypted_payload_save_writes_candidate_config_before_reporting_success() {
        let password = "password";
        let old_payload = SecretPayload::from_credentials(&[], "old-hydro", "old-hyper");
        let mut terminal = terminal_with_encrypted_payload(&old_payload, password);
        terminal.config_save_due_at = Some(std::time::Instant::now());
        let candidate = SecretPayload::from_credentials(&[], "new-hydro", "new-hyper");
        let save_called = Cell::new(false);

        let persisted = terminal.persist_encrypted_secret_payload_with(
            candidate,
            "Credentials saved to encrypted config",
            |cfg| {
                save_called.set(true);
                assert_eq!(
                    cfg.credential_storage_mode,
                    config::CredentialStorageMode::EncryptedConfig
                );
                let payload = config::decrypt_secrets(
                    cfg.encrypted_secrets
                        .as_ref()
                        .expect("candidate encrypted payload should be snapshotted"),
                    password,
                )
                .expect("candidate payload should decrypt");
                assert_eq!(payload.global_hydromancer_api_key(), "new-hydro");
                assert_eq!(payload.global_hyperdash_api_key(), "new-hyper");
                Ok(())
            },
        );

        assert!(persisted);
        assert!(save_called.get());
        let saved_payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted payload should remain committed"),
            password,
        )
        .expect("committed payload should decrypt");
        assert_eq!(saved_payload.global_hydromancer_api_key(), "new-hydro");
        assert_eq!(saved_payload.global_hyperdash_api_key(), "new-hyper");
        assert_eq!(
            terminal.secret_store_status,
            Some(("Credentials saved to encrypted config".to_string(), false))
        );
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn encrypted_payload_config_save_failure_rolls_back_candidate_blob() {
        let password = "password";
        let old_payload = SecretPayload::from_credentials(&[], "old-hydro", "old-hyper");
        let mut terminal = terminal_with_encrypted_payload(&old_payload, password);
        let original_encrypted = terminal.encrypted_secrets.clone();
        let candidate = SecretPayload::from_credentials(&[], "new-hydro", "new-hyper");

        let persisted = terminal.persist_encrypted_secret_payload_with(
            candidate,
            "Credentials saved to encrypted config",
            |_| Err("disk full".to_string()),
        );

        assert!(!persisted);
        assert_eq!(terminal.encrypted_secrets, original_encrypted);
        assert!(terminal.encrypted_secrets_unlocked);
        let saved_payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("old encrypted payload should remain committed"),
            password,
        )
        .expect("old payload should decrypt");
        assert_eq!(saved_payload.global_hydromancer_api_key(), "old-hydro");
        assert_eq!(saved_payload.global_hyperdash_api_key(), "old-hyper");
        assert!(terminal.secret_migration_save_blocked);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Encrypted credential config save failed"));
        assert!(message.contains("credential change was not committed"));
        assert!(message.contains("disk full"));
    }

    #[test]
    fn encrypted_payload_post_commit_save_warning_keeps_candidate_blob() {
        let password = "password";
        let old_payload = SecretPayload::from_credentials(&[], "old-hydro", "");
        let mut terminal = terminal_with_encrypted_payload(&old_payload, password);
        let candidate = SecretPayload::from_credentials(&[], "new-hydro", "");

        let persisted = terminal.persist_encrypted_secret_payload_with(
            candidate,
            "Credentials saved to encrypted config",
            |_| Err(config::installed_config_save_error_for_test("sync denied")),
        );

        assert!(persisted);
        let saved_payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("candidate encrypted payload should remain active"),
            password,
        )
        .expect("candidate payload should decrypt");
        assert_eq!(saved_payload.global_hydromancer_api_key(), "new-hydro");
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Credentials saved to encrypted config"));
        assert!(message.contains("config durability could not be fully verified"));
        assert!(!message.contains("not committed"));
    }

    #[test]
    fn encrypted_payload_save_retry_is_not_blocked_by_previous_secret_failure() {
        let password = "password";
        let old_payload = SecretPayload::from_credentials(&[], "old-hydro", "");
        let mut terminal = terminal_with_encrypted_payload(&old_payload, password);
        terminal.secret_migration_save_blocked = true;
        let candidate = SecretPayload::from_credentials(&[], "new-hydro", "");
        let save_called = Cell::new(false);

        let persisted = terminal.persist_encrypted_secret_payload_with(
            candidate,
            "Credentials saved to encrypted config",
            |_| {
                save_called.set(true);
                Ok(())
            },
        );

        assert!(persisted);
        assert!(save_called.get());
        assert!(!terminal.secret_migration_save_blocked);
    }

    #[test]
    fn encrypted_payload_apply_skips_wallet_binding_mismatch() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account(
            "acct-a",
            "0xdef0000000000000000000000000000000000000",
        )];
        terminal.active_account_index = 0;
        let payload = SecretPayload::from_credentials(
            &[AccountProfile {
                secret_id: "acct-a".to_string(),
                name: "acct-a".to_string(),
                wallet_address: "0xabc0000000000000000000000000000000000000".to_string(),
                agent_key: sensitive_string("agent-key").into_zeroizing(),
                hydromancer_api_key: sensitive_string("").into_zeroizing(),
            }],
            "",
            "",
        );

        let skipped = terminal.apply_secret_payload(payload);

        assert_eq!(skipped, 1);
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "");
        assert_eq!(terminal.wallet_key_input.as_str(), "");
    }

    #[test]
    fn encrypted_payload_apply_keeps_hydromancer_stream_state_when_key_is_unchanged() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.hydromancer_api_key = sensitive_string("hydro-secret");
        terminal.hydromancer_key_input = sensitive_string("hydro-secret");
        terminal.hydromancer_key_generation = 991_000_004;
        terminal.hyperdash_api_key = sensitive_string("hyper-secret");
        terminal.liquidations_last_rx_ms = Some(11);
        terminal.tracked_trades_last_rx_ms = Some(22);
        terminal.liquidations_reconnect_nonce = 3;
        terminal.tracked_trades_reconnect_nonce = 5;
        terminal.liquidations_status = "Liquidations current".to_string();
        terminal.tracked_trades_status = "Trades current".to_string();
        terminal.journal.snapshot_requests.insert(
            "perp:BTC:test".to_string(),
            JournalTradeSnapshotRequest {
                account_key: Some("acct".to_string()),
                address: "0xabc".to_string(),
                trade_id: "perp:BTC:test".to_string(),
                coin: "BTC".to_string(),
                source: ChartBackfillSource::Hydromancer,
                read_data_provider_generation: terminal.read_data_provider_generation,
                hydromancer_key_generation: terminal.hydromancer_key_generation,
                timeframe: Timeframe::M1,
                ladder_index: 0,
                trade_start_ms: 1_000,
                trade_end_ms: 2_000,
                is_open: false,
                start_ms: 0,
                end_ms: 3_000,
            },
        );
        terminal
            .journal
            .expanded_snapshot_trade_ids
            .insert("perp:BTC:test".to_string());
        let payload = SecretPayload::from_credentials(&[], "hydro-secret", "hyper-secret");

        let sent_manager_reconnect = ws::hydromancer_manager_reconnect_sent_for_test(
            hydromancer_stream_key(&terminal),
            || {
                let _skipped = terminal.apply_secret_payload(payload);
            },
        );

        assert!(!sent_manager_reconnect);
        assert_eq!(terminal.hydromancer_api_key.as_str(), "hydro-secret");
        assert_eq!(terminal.hydromancer_key_generation, 991_000_004);
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
    }

    #[test]
    fn encrypted_unlock_migrates_legacy_unbound_profile_key_to_wallet_binding() {
        let current_wallet = "0xabc0000000000000000000000000000000000000";
        let other_wallet = "0xdef0000000000000000000000000000000000000";
        let password = "password";
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account("acct-a", current_wallet)];
        terminal.active_account_index = 0;
        terminal.encrypted_secret_password = sensitive_string(password);
        let mut payload = SecretPayload::from_credentials(&[], "", "");
        assert!(payload.upsert_profile_agent_key("acct-a", "agent-key"));
        terminal.encrypted_secrets =
            Some(config::encrypt_secrets(&payload, password).expect("encrypt fixture"));

        let saved_payload = RefCell::new(None);
        let _task = terminal.unlock_encrypted_credentials_with(|cfg| {
            let payload = config::decrypt_secrets(
                cfg.encrypted_secrets
                    .as_ref()
                    .expect("migrated encrypted payload should be snapshotted"),
                password,
            )
            .expect("saved migrated payload should decrypt");
            saved_payload.replace(Some(payload));
            Ok(())
        });

        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert!(terminal.encrypted_secrets_unlocked);
        assert!(terminal.encrypted_secret_password.is_empty());
        assert!(terminal.encrypted_secret_confirm.is_empty());
        assert!(terminal.config_save_due_at.is_none());
        assert_eq!(
            terminal.secret_store_status,
            Some(("Encrypted credentials unlocked".to_string(), false))
        );
        let migrated_payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted payload should remain saved"),
            password,
        )
        .expect("migrated payload should decrypt");
        assert_eq!(
            migrated_payload.profile_agent_key_for_wallet("acct-a", current_wallet),
            Some("agent-key")
        );
        assert_eq!(
            migrated_payload.profile_agent_key_for_wallet("acct-a", other_wallet),
            None
        );
        let saved_payload = saved_payload
            .into_inner()
            .expect("migration should save immediately");
        assert_eq!(
            saved_payload.profile_agent_key_for_wallet("acct-a", current_wallet),
            Some("agent-key")
        );
        assert_eq!(
            saved_payload.profile_agent_key_for_wallet("acct-a", other_wallet),
            None
        );
    }

    #[test]
    fn encrypted_unlock_migration_save_failure_rolls_back_candidate_blob() {
        let current_wallet = "0xabc0000000000000000000000000000000000000";
        let other_wallet = "0xdef0000000000000000000000000000000000000";
        let password = "password";
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account("acct-a", current_wallet)];
        terminal.active_account_index = 0;
        terminal.encrypted_secret_password = sensitive_string(password);
        let mut payload = SecretPayload::from_credentials(&[], "", "");
        assert!(payload.upsert_profile_agent_key("acct-a", "agent-key"));
        terminal.encrypted_secrets =
            Some(config::encrypt_secrets(&payload, password).expect("encrypt fixture"));
        let original_encrypted = terminal.encrypted_secrets.clone();

        let _task = terminal.unlock_encrypted_credentials_with(|_| Err("disk full".to_string()));

        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert!(terminal.encrypted_secrets_unlocked);
        assert_eq!(terminal.encrypted_secrets, original_encrypted);
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        assert!(terminal.encrypted_secret_password.is_empty());
        assert!(terminal.encrypted_secret_confirm.is_empty());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Encrypted credentials unlocked"));
        assert!(message.contains("legacy wallet binding migration failed"));
        assert!(message.contains("disk full"));
        let rolled_back_payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("original encrypted payload should remain"),
            password,
        )
        .expect("original payload should decrypt");
        assert_eq!(
            rolled_back_payload.profile_agent_key("acct-a"),
            Some("agent-key")
        );
        assert_eq!(
            rolled_back_payload.profile_agent_key_for_wallet("acct-a", current_wallet),
            Some("agent-key")
        );
        assert_eq!(
            rolled_back_payload.profile_agent_key_for_wallet("acct-a", other_wallet),
            Some("agent-key")
        );
    }

    #[test]
    fn encrypted_unlock_retries_pending_keychain_cleanup_after_decrypt() {
        let current_wallet = "0xabc0000000000000000000000000000000000000";
        let password = "password";
        let payload = SecretPayload::from_credentials(
            &[AccountProfile {
                secret_id: "acct-a".to_string(),
                name: "acct-a".to_string(),
                wallet_address: current_wallet.to_string(),
                agent_key: sensitive_string("agent-key").into_zeroizing(),
                hydromancer_api_key: sensitive_string("").into_zeroizing(),
            }],
            "",
            "",
        );
        let mut terminal = terminal_with_encrypted_payload(&payload, password);
        terminal.accounts = vec![account("acct-a", current_wallet)];
        terminal.active_account_index = 0;
        terminal.pending_keychain_cleanup_all = true;
        terminal
            .pending_keychain_profile_deletions
            .push("acct-b".to_string());
        let cleaned_profiles = RefCell::new(Vec::new());
        let saved_snapshots = RefCell::new(Vec::new());

        let _task = terminal.unlock_encrypted_credentials_with_hooks(
            |cfg| {
                saved_snapshots.borrow_mut().push(cfg.clone());
                Ok(())
            },
            |profiles| {
                cleaned_profiles.replace(
                    profiles
                        .iter()
                        .map(|profile| profile.secret_id.clone())
                        .collect(),
                );
                Ok(())
            },
            |_| panic!("full cleanup should include pending profile deletions"),
        );

        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert!(!terminal.pending_keychain_cleanup_all);
        assert!(terminal.pending_keychain_profile_deletions.is_empty());
        assert_eq!(
            cleaned_profiles.borrow().as_slice(),
            ["acct-a".to_string(), "acct-b".to_string()]
        );
        assert_eq!(saved_snapshots.borrow().len(), 1);
        let saved_snapshots = saved_snapshots.borrow();
        let snapshot = &saved_snapshots[0];
        assert!(!snapshot.pending_keychain_cleanup_all);
        assert!(snapshot.pending_keychain_profile_deletions.is_empty());
        assert_eq!(
            terminal.secret_store_status,
            Some(("Encrypted credentials unlocked".to_string(), false))
        );
    }

    #[test]
    fn encrypted_unlock_pending_profile_cleanup_failure_redacts_trimmed_pending_id() {
        let current_wallet = "0xabc0000000000000000000000000000000000000";
        let password = "password";
        let payload = SecretPayload::from_credentials(
            &[AccountProfile {
                secret_id: "acct-a".to_string(),
                name: "acct-a".to_string(),
                wallet_address: current_wallet.to_string(),
                agent_key: sensitive_string("agent-key").into_zeroizing(),
                hydromancer_api_key: sensitive_string("").into_zeroizing(),
            }],
            "",
            "",
        );
        let mut terminal = terminal_with_encrypted_payload(&payload, password);
        terminal.accounts = vec![account("acct-a", current_wallet)];
        terminal.active_account_index = 0;
        terminal
            .pending_keychain_profile_deletions
            .push(" acct-b ".to_string());

        let _task = terminal.unlock_encrypted_credentials_with_hooks(
            |_| panic!("failed cleanup should not save cleared intent"),
            |_| panic!("profile-only cleanup must not run full cleanup"),
            |_secret_id| Err("delete acct-b denied".to_string()),
        );

        assert_eq!(
            terminal.pending_keychain_profile_deletions.as_slice(),
            [" acct-b "]
        );
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Encrypted credentials unlocked"));
        assert!(message.contains("<redacted-profile>"));
        assert!(!message.contains("acct-b"));
    }

    #[test]
    fn encrypted_unlock_requires_password_reentry_for_later_encrypted_save() {
        let password = "password";
        let payload = SecretPayload::from_credentials(&[], "old-hydro", "");
        let mut terminal = terminal_with_encrypted_payload(&payload, password);
        terminal.encrypted_secrets_unlocked = false;
        let original_encrypted = terminal.encrypted_secrets.clone();

        let _task = terminal.unlock_encrypted_credentials();

        assert!(terminal.encrypted_secrets_unlocked);
        assert!(terminal.encrypted_secret_password.is_empty());

        let persisted = terminal.persist_encrypted_secret_payload(
            SecretPayload::from_credentials(&[], "new-hydro", ""),
            "Credentials saved to encrypted config",
        );

        assert!(!persisted);
        assert_eq!(terminal.encrypted_secrets, original_encrypted);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Enter a password"));
    }

    #[test]
    fn encrypted_unlock_replaces_stale_hydromancer_chart_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.chart_backfill_source = ChartBackfillSource::Hydromancer;
        terminal.hydromancer_api_key = sensitive_string("");
        terminal.hydromancer_key_input = sensitive_string("");
        terminal.encrypted_secret_password = sensitive_string("password");
        terminal.charts.clear();

        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        instance.candle_fetch_request = Some(CandleFetchRequest {
            chart_id: 7,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            source: ChartBackfillSource::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        });
        terminal.charts.insert(7, instance);

        let payload =
            SecretPayload::from_credentials(&terminal.accounts, "new-hydromancer-key", "");
        terminal.encrypted_secrets =
            Some(config::encrypt_secrets(&payload, &terminal.encrypted_secret_password).unwrap());

        let _task = terminal.unlock_encrypted_credentials();

        assert_eq!(terminal.hydromancer_key_generation, 1);
        let request = terminal.charts[&7]
            .candle_fetch_request
            .as_ref()
            .expect("fresh candle request");
        assert_eq!(request.source, ChartBackfillSource::Hydromancer);
        assert_eq!(
            request.hydromancer_key_generation,
            terminal.hydromancer_key_generation
        );
    }
}
