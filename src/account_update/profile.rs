use crate::account_state::ActiveAccountSource;
use crate::account_state::PositionsSortColumn;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn update_positions_sort(&mut self, col: PositionsSortColumn) -> Task<Message> {
        if self.positions_sort_column == col {
            self.positions_sort_direction = match self.positions_sort_direction {
                config::SortDirection::Ascending => config::SortDirection::Descending,
                config::SortDirection::Descending => config::SortDirection::Ascending,
            };
        } else {
            self.positions_sort_column = col;
            self.positions_sort_direction = col.default_direction();
        }
        Task::none()
    }

    pub(super) fn toggle_hidden_position(&mut self, coin: String) -> Task<Message> {
        let Some(account_key) = self.active_journal_account_key() else {
            return Task::none();
        };

        let now_empty = {
            let hidden = self
                .hidden_positions_by_account
                .entry(account_key.clone())
                .or_default();
            if !hidden.insert(coin.clone()) {
                hidden.remove(&coin);
            }
            hidden.is_empty()
        };
        if now_empty {
            self.hidden_positions_by_account.remove(&account_key);
            self.show_hidden_positions = false;
        }
        if self.close_menu_coin.as_deref() == Some(coin.as_str()) {
            self.close_menu_coin = None;
        }
        if !self.ghost_account_secret_ids.contains(&account_key) {
            self.persist_config();
        }
        Task::none()
    }

    pub(super) fn toggle_show_hidden_positions(&mut self) -> Task<Message> {
        self.show_hidden_positions = !self.show_hidden_positions;
        Task::none()
    }

    pub(super) fn update_wallet_key_input(
        &mut self,
        value: crate::message::SecretInput,
    ) -> Task<Message> {
        let next_key = value.into_zeroizing();
        let key_changed = self.wallet_key_input.trim() != next_key.trim();
        if key_changed
            && (self.account_change_blocked_by_pending_trading_request("changing the agent key")
                || self.account_change_blocked_by_active_automation("changing the agent key"))
        {
            return Task::none();
        }

        if self.active_account_is_ghost() {
            self.wallet_key_input.zeroize();
            if let Some(profile) = self.accounts.get_mut(self.active_account_index) {
                profile.agent_key.zeroize();
            }
        } else {
            self.wallet_key_input.zeroize();
            self.wallet_key_input = next_key.into();
        }
        Task::none()
    }

    pub(super) fn update_wallet_address_input(&mut self, value: String) -> Task<Message> {
        #[cfg(not(test))]
        {
            self.update_wallet_address_input_with_hooks(
                value,
                config::save_config,
                |terminal, accounts, removed_profile_secret_id| {
                    terminal.persist_profile_agent_key_removal_from_accounts(
                        accounts,
                        removed_profile_secret_id,
                    )
                },
            )
        }
        #[cfg(test)]
        {
            self.update_wallet_address_input_with_hooks(
                value,
                |_| Ok(()),
                |terminal, accounts, removed_profile_secret_id| {
                    terminal.persist_profile_agent_key_removal_from_accounts(
                        accounts,
                        removed_profile_secret_id,
                    )
                },
            )
        }
    }

    fn update_wallet_address_input_with_hooks(
        &mut self,
        value: String,
        mut save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
        mut persist_profile_agent_key_removal: impl FnMut(
            &mut Self,
            &[config::AccountProfile],
            &str,
        ) -> bool,
    ) -> Task<Message> {
        let previous_normalized = self
            .accounts
            .get(self.active_account_index)
            .and_then(|profile| Self::normalize_wallet_address(&profile.wallet_address));
        let next_normalized = Self::normalize_wallet_address(&value);
        let address_binding_changed = previous_normalized != next_normalized;
        let selected_cluster_profile_binding_changed = address_binding_changed
            && self
                .accounts
                .get(self.active_account_index)
                .is_some_and(|profile| {
                    self.selected_wallet_cluster_uses_profile(&profile.secret_id)
                });
        let is_ghost = self.active_account_is_ghost();
        if !is_ghost
            && address_binding_changed
            && (self
                .account_change_blocked_by_pending_trading_request("changing the wallet address")
                || self.account_change_blocked_by_active_automation("changing the wallet address"))
        {
            return Task::none();
        }

        if self.active_account_index >= self.accounts.len() {
            self.wallet_address_input = value;
            return Task::none();
        }

        let had_agent_key = !self.wallet_key_input.trim().is_empty()
            || !self.accounts[self.active_account_index]
                .agent_key
                .trim()
                .is_empty();
        let should_remove_agent_key = !is_ghost && address_binding_changed;
        if should_remove_agent_key {
            let mut previous_wallet_address_input =
                std::mem::replace(&mut self.wallet_address_input, value.clone());
            let mut removed_profile_secret_id =
                self.accounts[self.active_account_index].secret_id.clone();
            let rollback = self
                .begin_active_profile_address_rebind(self.active_account_index, value.clone())
                .expect("active profile must exist after the address-edit bounds check");

            let mut os_metadata_saved = false;
            let persisted = {
                let persisted_accounts = self.persisted_accounts_snapshot();
                match self.secret_storage_mode {
                    config::CredentialStorageMode::OsKeychain => {
                        match self.persist_config_immediately_with(&mut save_config) {
                            Ok(()) => {
                                os_metadata_saved = true;
                                persist_profile_agent_key_removal(
                                    self,
                                    &persisted_accounts,
                                    &removed_profile_secret_id,
                                )
                            }
                            Err(error) => {
                                let error = redact_sensitive_response_text(&error);
                                self.secret_store_status = Some((
                                    format!(
                                        "Config save failed: {error}. Wallet address was not changed; retry after config persistence is available."
                                    ),
                                    true,
                                ));
                                false
                            }
                        }
                    }
                    config::CredentialStorageMode::EncryptedConfig => {
                        persist_profile_agent_key_removal(
                            self,
                            &persisted_accounts,
                            &removed_profile_secret_id,
                        )
                    }
                }
            };
            removed_profile_secret_id.zeroize();
            if !persisted {
                let detail = self
                    .secret_store_status
                    .as_ref()
                    .map(|(message, _)| message.clone())
                    .unwrap_or_else(|| "Credential storage update failed".to_string());
                let detail = redact_sensitive_response_text(&detail);
                rollback.restore(self, previous_wallet_address_input);
                if self.secret_storage_mode == config::CredentialStorageMode::OsKeychain
                    && os_metadata_saved
                    && let Err(error) = self
                        .persist_config_immediately_for_secret_migration_rollback_with(
                            &mut save_config,
                        )
                {
                    let error = redact_sensitive_response_text(&error);
                    self.secret_store_status = Some((
                        format!(
                            "{detail}. Wallet address was not changed, but saving the rollback failed: {error}. Retry after config persistence is available."
                        ),
                        true,
                    ));
                    return Task::none();
                }
                self.secret_store_status = Some((
                    format!(
                        "{detail}. Wallet address was not changed; retry after credential storage is available."
                    ),
                    true,
                ));
                return Task::none();
            }
            rollback.scrub_after_commit();
            previous_wallet_address_input.zeroize();
        }

        self.wallet_address_input = value;
        self.accounts[self.active_account_index].wallet_address = self.wallet_address_input.clone();
        if address_binding_changed {
            if selected_cluster_profile_binding_changed {
                self.rotate_wallet_cluster_user_data_streams();
            }
            self.clear_percentage_order_quantity();
        }
        if should_remove_agent_key {
            let cleanup_warning = self
                .secret_store_status
                .as_ref()
                .and_then(|(message, is_error)| (*is_error).then(|| message.clone()));
            self.wallet_key_input.zeroize();
            self.accounts[self.active_account_index].agent_key.zeroize();
            let mut status = if had_agent_key {
                "Agent key cleared for this account and removed from credential storage; re-enter and save credentials for the new wallet address to trade."
                    .to_string()
            } else {
                "Saved agent key binding removed from credential storage for this account; save credentials for the new wallet address before trading."
                    .to_string()
            };
            if let Some(cleanup_warning) = cleanup_warning {
                status.push(' ');
                status.push_str(&cleanup_warning);
            }
            self.secret_store_status = Some((status, true));
            return Task::none();
        }
        if !is_ghost {
            self.persist_config();
        }
        Task::none()
    }

    fn update_account_label_at_index(&mut self, index: usize, value: String) -> Task<Message> {
        let is_ghost = self.account_index_is_ghost(index);
        if let Some(profile) = self.accounts.get_mut(index) {
            profile.name = value;
            if !is_ghost {
                self.persist_config();
            }
        }
        Task::none()
    }

    pub(super) fn toggle_account_picker(&mut self) -> Task<Message> {
        let opening = !self.account_picker_open;
        if opening {
            self.close_chart_header_menus();
        } else {
            self.account_picker_rename_index = None;
        }
        self.account_picker_open = opening;
        Task::none()
    }

    pub(super) fn select_account_from_picker(&mut self, index: usize) -> Task<Message> {
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.active_account_source = ActiveAccountSource::Hyperliquid;
        self.switch_account_task(index)
    }

    pub(super) fn toggle_account_picker_rename(&mut self, index: usize) -> Task<Message> {
        if self.accounts.get(index).is_none() || self.account_picker_rename_index == Some(index) {
            self.account_picker_rename_index = None;
        } else {
            self.account_picker_rename_index = Some(index);
        }
        Task::none()
    }

    pub(super) fn update_account_picker_label(
        &mut self,
        index: usize,
        value: String,
    ) -> Task<Message> {
        if self.accounts.get(index).is_none() {
            self.account_picker_rename_index = None;
            return Task::none();
        }
        self.account_picker_rename_index = Some(index);
        self.update_account_label_at_index(index, value)
    }

    pub(super) fn add_ghost_wallet_from_picker(&mut self, address: String) -> Task<Message> {
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.ghost_wallet_task(address)
    }

    pub(super) fn forget_ghost_account_from_picker(&mut self, index: usize) -> Task<Message> {
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.forget_ghost_account_task(index)
    }

    pub(super) fn save_active_account_credentials(&mut self) -> Task<Message> {
        self.save_active_account_credentials_with(|terminal, accounts| {
            terminal.persist_active_profile_secrets_from_accounts(accounts)
        })
    }

    fn save_active_account_credentials_with(
        &mut self,
        mut persist_profile_secrets: impl FnMut(&mut Self, &[config::AccountProfile]) -> bool,
    ) -> Task<Message> {
        if self.active_account_is_ghost() {
            self.secret_store_status = Some(("Ghost wallets are in memory only".into(), false));
            return Task::none();
        }
        if self.active_account_index >= self.accounts.len() {
            return Task::none();
        }

        let key_changed = self.wallet_key_input.trim()
            != self.accounts[self.active_account_index].agent_key.trim();
        if key_changed
            && (self.account_change_blocked_by_pending_trading_request("saving the agent key")
                || self.account_change_blocked_by_active_automation("saving the agent key"))
        {
            return Task::none();
        }

        let profile_index = self.active_account_index;
        let (mut persisted_accounts, persisted_profile_index) = self
            .persisted_accounts_with_active_agent_key(self.wallet_key_input.as_str())
            .expect("active saved profile must exist while staging credentials");

        let persisted = persist_profile_secrets(self, &persisted_accounts);
        if persisted {
            let staged_profile = persisted_accounts
                .get_mut(persisted_profile_index)
                .expect("staged active profile disappeared after credential persistence");
            let profile = self
                .accounts
                .get_mut(profile_index)
                .expect("active profile disappeared after credential persistence");
            assert!(
                profile.secret_id == staged_profile.secret_id,
                "active profile identity changed during credential persistence"
            );
            profile.agent_key.zeroize();
            profile.agent_key = std::mem::take(&mut staged_profile.agent_key);
        }
        drop(persisted_accounts);

        if persisted {
            self.persist_config();
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::sensitive_string;
    use crate::config::{self, AccountProfile};
    use crate::order_execution::PendingOrderAction;
    use crate::signing::{ChaseLifecycle, ChaseOrder};
    use crate::twap_state::{TwapOrder, TwapOrderInit, TwapStatus};

    use std::cell::{Cell, RefCell};
    use std::time::{Duration, Instant};

    const ADDRESS_A: &str = "0xabc0000000000000000000000000000000000000";
    const ADDRESS_B: &str = "0xdef0000000000000000000000000000000000000";

    fn account(secret_id: &str, wallet_address: &str, agent_key: &str) -> AccountProfile {
        AccountProfile {
            secret_id: secret_id.to_string(),
            name: secret_id.to_string(),
            wallet_address: wallet_address.to_string(),
            agent_key: sensitive_string(agent_key).into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }
    }

    fn terminal_with_active_account(wallet_address: &str, agent_key: &str) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.desktop_notifications = false;
        terminal.accounts = vec![account("acct-a", wallet_address, agent_key)];
        terminal.active_account_index = 0;
        terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
        terminal.wallet_key_input = terminal.accounts[0].agent_key.clone().into();
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secret_password = sensitive_string("test-password");
        terminal.encrypted_secrets_unlocked = true;
        terminal.encrypted_secrets = Some(
            config::encrypt_secrets(
                &config::SecretPayload::from_credentials(&terminal.accounts, "", ""),
                &terminal.encrypted_secret_password,
            )
            .expect("test encrypted payload"),
        );
        terminal.secret_store_status = None;
        terminal
    }

    fn chase_order(account_address: &str) -> ChaseOrder {
        ChaseOrder {
            id: 42,
            coin: "BTC".to_string(),
            account_address: account_address.to_string(),
            agent_key: sensitive_string("agent-key").into_zeroizing().into(),
            is_buy: true,
            target_size: 1.0,
            filled_size: 0.0,
            remaining_size: 1.0,
            known_oids: vec![1001],
            current_cloid: None,
            place_attempt_count: 0,
            asset: 0,
            sz_decimals: 5,
            is_spot: false,
            reduce_only: false,
            current_oid: Some(1001),
            current_price: 50_000.0,
            current_price_wire: "50000".to_string(),
            initial_price: 50_000.0,
            started_at: Instant::now(),
            started_at_ms: 1,
            fill_cutoff_ms_by_oid: Vec::new(),
            reprice_count: 0,
            lifecycle: ChaseLifecycle::Resting,
            last_reprice_at: None,
            desired_price: None,
            stop_reason: None,
            cancel_retries: 0,
        }
    }

    fn twap_order(id: u64, account_address: &str) -> TwapOrder {
        let now = Instant::now();
        TwapOrder::new(TwapOrderInit {
            id,
            coin: "BTC".to_string(),
            display_coin: "BTC".to_string(),
            account_address: account_address.to_string(),
            agent_key: sensitive_string("agent-key").into_zeroizing().into(),
            is_buy: true,
            target_size: 1.0,
            asset: 0,
            sz_decimals: 5,
            is_spot: false,
            reduce_only: false,
            min_price: 49_000.0,
            max_price: 51_000.0,
            randomize: false,
            duration: Duration::from_secs(60),
            slice_count: 1,
            now,
            started_at_ms: TradingTerminal::now_ms(),
        })
    }

    #[test]
    fn account_picker_label_message_preserves_exact_text_and_persistence() {
        const LABEL: &str = "  Private Account Label  ";
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.config_save_due_at = None;

        let _task = terminal.update_account(Message::AccountPickerLabelChanged(0, LABEL.into()));

        assert_eq!(terminal.accounts[0].name, LABEL);
        assert_eq!(terminal.account_picker_rename_index, Some(0));
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn wallet_address_edit_clears_agent_key_when_binding_changes() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");

        let _task = terminal.update_wallet_address_input(ADDRESS_B.to_string());

        assert_eq!(terminal.wallet_key_input.as_str(), "");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "");
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_B);
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should be rewritten"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.profile_agent_key("acct-a"), None);
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Agent key cleared"));
    }

    #[test]
    fn wallet_address_edit_is_rejected_when_encrypted_credentials_are_locked() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.encrypted_secrets_unlocked = false;
        terminal.config_save_due_at = None;
        let original_encrypted = terminal.encrypted_secrets.clone();
        let profile_key_allocation = terminal.accounts[0].agent_key.as_ptr();
        let input_key_allocation = terminal.wallet_key_input.as_str().as_ptr();

        let _task = terminal.update_wallet_address_input(ADDRESS_B.to_string());

        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            profile_key_allocation
        );
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            input_key_allocation
        );
        assert_eq!(terminal.encrypted_secrets, original_encrypted);
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Unlock encrypted credentials"));
        assert!(message.contains("Wallet address was not changed"));
    }

    #[test]
    fn wallet_address_edit_is_rejected_while_config_save_is_in_flight() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        terminal.config_save_in_flight = true;

        let _task = terminal.update_wallet_address_input(ADDRESS_B.to_string());

        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert!(terminal.config_save_in_flight);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Config save failed"));
        assert!(message.contains("Wallet address was not changed"));
    }

    #[test]
    fn wallet_address_edit_config_save_failure_redacts_status() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        let keychain_called = Cell::new(false);

        let _task = terminal.update_wallet_address_input_with_hooks(
            ADDRESS_B.to_string(),
            |_cfg| Err("write failed: api_key=profile-secret".to_string()),
            |_terminal, _accounts, _removed_profile_secret_id| {
                keychain_called.set(true);
                true
            },
        );

        assert!(!keychain_called.get());
        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("api_key=<redacted>"));
        assert!(!message.contains("profile-secret"));
        assert!(message.contains("Wallet address was not changed"));
    }

    #[test]
    fn wallet_address_edit_installed_snapshot_error_preserves_failure_behavior_pending_policy() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        let keychain_called = Cell::new(false);
        let profile_key_allocation = terminal.accounts[0].agent_key.as_ptr();
        let input_key_allocation = terminal.wallet_key_input.as_str().as_ptr();

        let _task = terminal.update_wallet_address_input_with_hooks(
            ADDRESS_B.to_string(),
            |snapshot| {
                assert_eq!(snapshot.accounts[0].wallet_address, ADDRESS_B);
                Err(config::installed_config_save_error_for_test(
                    "sync failed: api_key=profile-secret",
                ))
            },
            |_terminal, _accounts, _removed_profile_secret_id| {
                keychain_called.set(true);
                true
            },
        );

        assert!(!keychain_called.get());
        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            profile_key_allocation
        );
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            input_key_allocation
        );
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Config save failed"));
        assert!(message.contains("api_key=<redacted>"));
        assert!(!message.contains("profile-secret"));
        assert!(message.contains("Wallet address was not changed"));
    }

    #[test]
    fn wallet_address_edit_os_keychain_failure_rolls_back_saved_metadata() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        let saved_snapshots = RefCell::new(Vec::<config::KeroseneConfig>::new());
        let keychain_called = Cell::new(false);
        let profile_key_allocation = terminal.accounts[0].agent_key.as_ptr();
        let input_key_allocation = terminal.wallet_key_input.as_str().as_ptr();

        let _task = terminal.update_wallet_address_input_with_hooks(
            ADDRESS_B.to_string(),
            |cfg| {
                saved_snapshots.borrow_mut().push(cfg.clone());
                Ok(())
            },
            |terminal, accounts, removed_profile_secret_id| {
                keychain_called.set(true);
                assert_eq!(removed_profile_secret_id, "acct-a");
                assert_eq!(accounts.len(), 1);
                assert_eq!(accounts[0].wallet_address, ADDRESS_B);
                assert!(accounts[0].agent_key.trim().is_empty());
                terminal.secret_migration_save_blocked = true;
                terminal.secret_store_status =
                    Some(("Keychain update failed: denied".into(), true));
                false
            },
        );

        assert!(keychain_called.get());
        let saved_snapshots = saved_snapshots.borrow();
        assert_eq!(saved_snapshots.len(), 2);
        assert_eq!(saved_snapshots[0].accounts[0].wallet_address, ADDRESS_B);
        assert!(saved_snapshots[0].accounts[0].agent_key.trim().is_empty());
        assert_eq!(saved_snapshots[1].accounts[0].wallet_address, ADDRESS_A);
        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            profile_key_allocation
        );
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            input_key_allocation
        );
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Keychain update failed: denied"));
        assert!(message.contains("Wallet address was not changed"));
        assert!(!message.contains("rollback failed"));
    }

    #[test]
    fn wallet_address_edit_rollback_save_failure_redacts_status() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        let save_count = Cell::new(0);

        let _task = terminal.update_wallet_address_input_with_hooks(
            ADDRESS_B.to_string(),
            |_cfg| {
                let count = save_count.get();
                save_count.set(count + 1);
                if count == 0 {
                    Ok(())
                } else {
                    Err("rollback failed: signature=rollback-secret".to_string())
                }
            },
            |terminal, _accounts, _removed_profile_secret_id| {
                terminal.secret_store_status = Some((
                    "Keychain update failed: auth_token=keychain-secret".to_string(),
                    true,
                ));
                false
            },
        );

        assert_eq!(save_count.get(), 2);
        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("auth_token=<redacted>"));
        assert!(message.contains("signature=<redacted>"));
        assert!(!message.contains("keychain-secret"));
        assert!(!message.contains("rollback-secret"));
        assert!(message.contains("Wallet address was not changed"));
    }

    #[test]
    fn wallet_address_case_edit_preserves_agent_key_for_same_address() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");

        let _task = terminal
            .update_wallet_address_input("0xABC0000000000000000000000000000000000000".to_string());

        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert_eq!(
            terminal.accounts[0].wallet_address,
            "0xABC0000000000000000000000000000000000000"
        );
    }

    #[test]
    fn wallet_key_input_change_does_not_commit_agent_key_before_save() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.connected_address = Some(ADDRESS_A.to_string());

        let _task = terminal.update_wallet_key_input("new-agent-key".into());

        assert_eq!(terminal.wallet_key_input.as_str(), "new-agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        let (agent_key, account_address) = terminal
            .captured_order_signing_context()
            .expect("committed key should still sign");
        assert_eq!(agent_key.as_str(), "agent-key");
        assert_eq!(account_address, ADDRESS_A);
    }

    #[test]
    fn agent_key_save_commits_after_encrypted_persistence_succeeds() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");

        let _task = terminal.update_wallet_key_input("new-agent-key".into());
        let _task = terminal.save_active_account_credentials();

        assert_eq!(terminal.wallet_key_input.as_str(), "new-agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "new-agent-key");
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should be rewritten"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(
            payload.profile_agent_key_for_wallet("acct-a", ADDRESS_A),
            Some("new-agent-key")
        );
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn agent_key_save_commits_the_exact_persisted_snapshot_allocation() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal
            .accounts
            .insert(0, account("ghost-a", ADDRESS_B, "ghost-agent-key"));
        terminal
            .ghost_account_secret_ids
            .insert("ghost-a".to_string());
        terminal.active_account_index = 1;
        terminal
            .accounts
            .push(account("acct-b", ADDRESS_B, "other-agent-key"));
        let original_committed_allocation = terminal.accounts[1].agent_key.as_ptr();

        let _task = terminal.update_wallet_key_input("new-agent-key".into());
        let draft_allocation = terminal.wallet_key_input.as_str().as_ptr();
        let staged_allocation = Cell::new(std::ptr::null::<u8>());
        let _task = terminal.save_active_account_credentials_with(|terminal, accounts| {
            assert_eq!(terminal.accounts[1].agent_key.as_str(), "agent-key");
            assert_eq!(
                terminal.accounts[1].agent_key.as_ptr(),
                original_committed_allocation
            );
            assert_eq!(
                terminal.wallet_key_input.as_str().as_ptr(),
                draft_allocation
            );
            assert_eq!(accounts.len(), 2);
            assert_eq!(accounts[0].secret_id, "acct-a");
            assert_eq!(accounts[0].agent_key.as_str(), "new-agent-key");
            assert_eq!(accounts[1].secret_id, "acct-b");
            assert_eq!(accounts[1].agent_key.as_str(), "other-agent-key");
            staged_allocation.set(accounts[0].agent_key.as_ptr());
            true
        });

        assert_eq!(terminal.accounts[1].agent_key.as_str(), "new-agent-key");
        assert_eq!(
            terminal.accounts[1].agent_key.as_ptr(),
            staged_allocation.get()
        );
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            draft_allocation
        );
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn agent_key_save_failure_preserves_original_committed_and_draft_allocations() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.config_save_due_at = None;
        let original_committed_allocation = terminal.accounts[0].agent_key.as_ptr();

        let _task = terminal.update_wallet_key_input("new-agent-key".into());
        let draft_allocation = terminal.wallet_key_input.as_str().as_ptr();
        let _task = terminal.save_active_account_credentials_with(|terminal, accounts| {
            assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
            assert_eq!(accounts[0].agent_key.as_str(), "new-agent-key");
            false
        });

        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            original_committed_allocation
        );
        assert_eq!(terminal.wallet_key_input.as_str(), "new-agent-key");
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            draft_allocation
        );
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn agent_key_save_locked_encrypted_credentials_keeps_committed_signing_key() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.connected_address = Some(ADDRESS_A.to_string());
        terminal.encrypted_secrets_unlocked = false;
        terminal.config_save_due_at = None;
        let original_encrypted = terminal.encrypted_secrets.clone();

        let _task = terminal.update_wallet_key_input("new-agent-key".into());
        let _task = terminal.save_active_account_credentials();

        assert_eq!(terminal.wallet_key_input.as_str(), "new-agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert_eq!(terminal.encrypted_secrets, original_encrypted);
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (agent_key, account_address) = terminal
            .captured_order_signing_context()
            .expect("old committed key should still sign");
        assert_eq!(agent_key.as_str(), "agent-key");
        assert_eq!(account_address, ADDRESS_A);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Unlock encrypted credentials"));
    }

    #[test]
    fn agent_key_save_draft_change_is_blocked_while_chase_is_active() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.wallet_key_input = sensitive_string("new-agent-key");
        terminal.chase_orders.insert(42, chase_order(ADDRESS_A));

        let _task = terminal.save_active_account_credentials();

        assert_eq!(terminal.wallet_key_input.as_str(), "new-agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert!(terminal.config_save_due_at.is_none());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked key save should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("active chase orders"));
        assert!(toast.message.contains("saving the agent key"));
    }

    #[test]
    fn agent_key_save_draft_change_is_blocked_while_twap_is_active() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.wallet_key_input = sensitive_string("new-agent-key");
        terminal.twap_orders.insert(7, twap_order(7, ADDRESS_A));

        let _task = terminal.save_active_account_credentials();

        assert_eq!(terminal.wallet_key_input.as_str(), "new-agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert!(terminal.config_save_due_at.is_none());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked key save should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("active TWAP orders"));
        assert!(toast.message.contains("saving the agent key"));
    }

    #[test]
    fn agent_key_save_draft_change_is_blocked_while_pending_request_exists() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.wallet_key_input = sensitive_string("new-agent-key");
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.save_active_account_credentials();

        assert_eq!(terminal.wallet_key_input.as_str(), "new-agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert!(terminal.config_save_due_at.is_none());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked key save should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before saving the agent key")
        );
    }

    #[test]
    fn agent_key_save_unchanged_key_is_allowed_while_chase_is_active() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.chase_orders.insert(42, chase_order(ADDRESS_A));

        let _task = terminal.save_active_account_credentials();

        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert!(terminal.toasts.is_empty());
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn wallet_key_edit_is_blocked_while_twap_is_active() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.twap_orders.insert(7, twap_order(7, ADDRESS_A));

        let _task = terminal.update_wallet_key_input("new-agent-key".into());

        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        let toast = terminal
            .toasts
            .last()
            .expect("blocked key edit should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("active TWAP orders"));
        assert!(toast.message.contains("changing the agent key"));
    }

    #[test]
    fn wallet_key_edit_is_blocked_while_chase_is_active() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.chase_orders.insert(42, chase_order(ADDRESS_A));

        let _task = terminal.update_wallet_key_input("new-agent-key".into());

        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        let toast = terminal
            .toasts
            .last()
            .expect("blocked key edit should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("active chase orders"));
        assert!(toast.message.contains("changing the agent key"));
    }

    #[test]
    fn wallet_key_edit_is_blocked_while_pending_trading_request_exists() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.update_wallet_key_input("new-agent-key".into());

        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        let toast = terminal
            .toasts
            .last()
            .expect("blocked key edit should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before changing the agent key")
        );
    }

    #[test]
    fn wallet_address_edit_is_blocked_while_chase_is_active() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.chase_orders.insert(42, chase_order(ADDRESS_A));

        let _task = terminal.update_wallet_address_input(ADDRESS_B.to_string());

        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert!(terminal.secret_store_status.is_none());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked address edit should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("active chase orders"));
        assert!(toast.message.contains("changing the wallet address"));
    }

    #[test]
    fn wallet_address_edit_is_blocked_while_twap_is_active() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.twap_orders.insert(7, twap_order(7, ADDRESS_A));

        let _task = terminal.update_wallet_address_input(ADDRESS_B.to_string());

        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        assert!(terminal.secret_store_status.is_none());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked address edit should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("active TWAP orders"));
        assert!(toast.message.contains("changing the wallet address"));
    }

    #[test]
    fn wallet_address_edit_is_blocked_while_pending_trading_request_exists() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.update_wallet_address_input(ADDRESS_B.to_string());

        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_A);
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");
        let toast = terminal
            .toasts
            .last()
            .expect("blocked address edit should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before changing the wallet address")
        );
    }

    #[test]
    fn terminal_twap_does_not_block_credential_edits() {
        let mut terminal = terminal_with_active_account(ADDRESS_A, "agent-key");
        let mut completed = twap_order(7, ADDRESS_A);
        completed.status = TwapStatus::Completed;
        terminal.twap_orders.insert(7, completed);

        let _task = terminal.update_wallet_key_input("new-agent-key".into());

        assert_eq!(terminal.wallet_key_input.as_str(), "new-agent-key");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "agent-key");

        let _task = terminal.update_wallet_address_input(ADDRESS_B.to_string());

        assert_eq!(terminal.wallet_address_input, ADDRESS_B);
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_B);
        assert_eq!(terminal.wallet_key_input.as_str(), "");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "");
        assert!(terminal.toasts.is_empty());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Agent key cleared"));
    }
}
