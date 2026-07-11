use crate::account_state::{ActiveAccountSource, AddAccountWindowState};
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
use crate::message::{Message, SecretInput};
use crate::signing;

use iced::{Size, Task, window};
use zeroize::{Zeroize, Zeroizing};

// ---------------------------------------------------------------------------
// Add Account Window
// ---------------------------------------------------------------------------

const ADD_ACCOUNT_WINDOW_SIZE: Size = Size {
    width: 470.0,
    height: 560.0,
};
const ADD_ACCOUNT_WINDOW_MIN_SIZE: Size = Size {
    width: 420.0,
    height: 460.0,
};

/// Validated add-account values. Its key is the one caller-owned storage
/// staging allocation and moves into canonical state only after acceptance.
struct AddAccountSubmission {
    window_id: window::Id,
    switch_on_add: bool,
    profile: config::AccountProfile,
}

impl TradingTerminal {
    pub(super) fn open_add_account_window(&mut self) -> Task<Message> {
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        if let Some(state) = &self.add_account_window {
            return window::gain_focus(state.window_id);
        }

        let settings = window::Settings {
            size: ADD_ACCOUNT_WINDOW_SIZE,
            min_size: Some(ADD_ACCOUNT_WINDOW_MIN_SIZE),
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (id, task) = window::open(settings);
        self.add_account_window = Some(AddAccountWindowState::new(id));
        task.map(Message::WindowOpened)
    }

    pub(super) fn update_add_account_name(&mut self, value: String) -> Task<Message> {
        if let Some(state) = self.add_account_window.as_mut() {
            state.name_input = value;
        }
        Task::none()
    }

    pub(super) fn update_add_account_address(&mut self, value: String) -> Task<Message> {
        if let Some(state) = self.add_account_window.as_mut() {
            state.address_input = value;
            state.error = None;
        }
        Task::none()
    }

    pub(super) fn update_add_account_key(&mut self, value: SecretInput) -> Task<Message> {
        if let Some(state) = self.add_account_window.as_mut() {
            state.key_input.zeroize();
            state.key_input = value.into_zeroizing().into();
            state.error = None;
        }
        Task::none()
    }

    pub(super) fn toggle_add_account_switch(&mut self, value: bool) -> Task<Message> {
        if let Some(state) = self.add_account_window.as_mut() {
            state.switch_on_add = value;
        }
        Task::none()
    }

    pub(super) fn cancel_add_account_window(&mut self) -> Task<Message> {
        let Some(state) = self.add_account_window.take() else {
            return Task::none();
        };
        window::close(state.window_id)
    }

    pub(super) fn submit_add_account(&mut self) -> Task<Message> {
        self.submit_add_account_with(|terminal, accounts| {
            terminal.persist_profile_secrets_from_accounts(accounts)
        })
    }

    fn submit_add_account_with(
        &mut self,
        persist_profile_secrets: impl FnOnce(&mut Self, &[config::AccountProfile]) -> bool,
    ) -> Task<Message> {
        let submission = match self.prepare_add_account_submission() {
            Ok(Some(submission)) => submission,
            Ok(None) => return Task::none(),
            Err(error) => {
                self.set_add_account_error(error);
                return Task::none();
            }
        };

        let AddAccountSubmission {
            window_id,
            switch_on_add,
            profile,
        } = submission;
        let profile_name = profile.name.clone();
        let has_agent_key = !profile.agent_key.is_empty();

        let new_index = if has_agent_key {
            // The immediate encrypted-config save must see the new profile
            // metadata in canonical state. Keep that provisional shell unable
            // to sign while the only staged key is synchronously persisted.
            let mut persisted_accounts = self.persisted_accounts_snapshot();
            let staged_profile_index = persisted_accounts.len();
            let provisional_profile = config::AccountProfile {
                secret_id: profile.secret_id.clone(),
                name: profile.name.clone(),
                wallet_address: profile.wallet_address.clone(),
                agent_key: String::new().into(),
                hydromancer_api_key: String::new().into(),
            };
            persisted_accounts.push(profile);
            self.accounts.push(provisional_profile);
            let new_index = self.accounts.len() - 1;

            let migration_blocked_before = self.secret_migration_save_blocked;
            if !persist_profile_secrets(self, &persisted_accounts) {
                let removed_profile = self
                    .accounts
                    .pop()
                    .expect("provisional add-account profile must remain last");
                assert!(
                    removed_profile.secret_id == persisted_accounts[staged_profile_index].secret_id,
                    "provisional add-account profile identity changed during credential persistence"
                );
                drop(removed_profile);
                drop(persisted_accounts);
                // The failed write only tried to add a key that was never
                // committed anywhere else, so the last-saved config still
                // matches the credential store; don't leave config saves
                // paused on its account.
                self.secret_migration_save_blocked = migration_blocked_before;
                let detail = self
                    .secret_store_status
                    .as_ref()
                    .map(|(message, _)| message.clone())
                    .unwrap_or_else(|| "Credential storage update failed".to_string());
                let detail = redact_sensitive_response_text(&detail);
                self.set_add_account_error(format!("{detail}. The account was not added."));
                return Task::none();
            }

            let staged_profile = persisted_accounts
                .pop()
                .expect("persisted add-account profile must remain last");
            let canonical_slot = self
                .accounts
                .get_mut(new_index)
                .expect("provisional add-account profile must survive credential persistence");
            assert!(
                canonical_slot.secret_id == staged_profile.secret_id,
                "add-account profile identity changed during credential persistence"
            );
            let provisional_profile = std::mem::replace(canonical_slot, staged_profile);
            drop(provisional_profile);
            drop(persisted_accounts);
            new_index
        } else {
            self.accounts.push(profile);
            self.accounts.len() - 1
        };

        self.persist_config();
        let prepared_input_key = (new_index == self.active_account_index)
            .then(|| self.take_verified_add_account_key(new_index));
        self.add_account_window = None;
        let close_task = window::close(window_id);

        if new_index == self.active_account_index {
            // switch_account_task no-ops on the already-active index (the
            // empty boot slot when this is the first saved account), so the
            // active-account inputs have to be synced here.
            let profile = &self.accounts[new_index];
            let secret_id = profile.secret_id.clone();
            self.wallet_address_input = profile.wallet_address.clone();
            self.wallet_key_input.zeroize();
            self.wallet_key_input = prepared_input_key
                .expect("active add-account profile must retain its prepared key")
                .into();
            self.last_persisted_active_account_secret_id = Some(secret_id.clone());
            self.journal.switch_active_account(Some(secret_id));
            if switch_on_add {
                self.active_account_source = ActiveAccountSource::Hyperliquid;
                self.account_connect_pending = true;
                return Task::batch([close_task, Task::done(Message::ConnectWallet)]);
            }
            self.push_toast(format!("Added account \"{profile_name}\""), false);
            return close_task;
        }

        if switch_on_add {
            let switch_task = self.switch_account_task(new_index);
            if self.active_account_index == new_index {
                self.active_account_source = ActiveAccountSource::Hyperliquid;
            } else {
                self.push_toast(
                    format!("Added account \"{profile_name}\" without switching to it"),
                    false,
                );
            }
            return Task::batch([close_task, switch_task]);
        }

        self.push_toast(format!("Added account \"{profile_name}\""), false);
        close_task
    }

    fn prepare_add_account_submission(&self) -> Result<Option<AddAccountSubmission>, String> {
        let Some(state) = self.add_account_window.as_ref() else {
            return Ok(None);
        };
        let Some(address) = Self::normalize_wallet_address(&state.address_input) else {
            return Err(
                "Enter a valid master account address (0x followed by 40 hex characters)."
                    .to_string(),
            );
        };

        let agent_key = state.key_input.trim();
        if !agent_key.is_empty()
            && let Err(error) = signing::validate_agent_key(agent_key)
        {
            return Err(format!("Agent key cannot be used for trading: {error}"));
        }

        let name = state.name_input.trim().to_string();
        let saved_account_count = self
            .accounts
            .iter()
            .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
            .count();
        Ok(Some(AddAccountSubmission {
            window_id: state.window_id,
            switch_on_add: state.switch_on_add,
            profile: config::AccountProfile {
                secret_id: config::new_secret_id(),
                name: if name.is_empty() {
                    format!("Account {}", saved_account_count + 1)
                } else {
                    name
                },
                wallet_address: address,
                agent_key: agent_key.to_string().into(),
                hydromancer_api_key: String::new().into(),
            },
        }))
    }

    fn take_verified_add_account_key(&mut self, profile_index: usize) -> Zeroizing<String> {
        let raw_key = {
            let state = self
                .add_account_window
                .as_mut()
                .expect("successful add-account submission must retain its draft window");
            std::mem::take(&mut state.key_input).into_zeroizing()
        };
        let normalized_key = if raw_key.trim().len() == raw_key.len() {
            raw_key
        } else {
            Zeroizing::new(raw_key.trim().to_string())
        };
        let profile = self
            .accounts
            .get(profile_index)
            .expect("successful add-account profile must exist before draft transfer");
        if profile.agent_key.as_str() == normalized_key.as_str() {
            normalized_key
        } else {
            profile.agent_key.clone()
        }
    }

    fn set_add_account_error(&mut self, message: impl Into<String>) {
        if let Some(state) = self.add_account_window.as_mut() {
            state.error = Some(message.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::sensitive_string;
    use crate::config::AccountProfile;
    use crate::signing::{ChaseLifecycle, ChaseOrder};

    use std::cell::Cell;
    use std::time::Instant;

    const ADDRESS_A: &str = "0xabc0000000000000000000000000000000000000";
    const ADDRESS_B: &str = "0xdef0000000000000000000000000000000000000";
    const VALID_KEY: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";
    const OTHER_VALID_KEY: &str =
        "0x0000000000000000000000000000000000000000000000000000000000000002";

    fn account(secret_id: &str, wallet_address: &str, agent_key: &str) -> AccountProfile {
        AccountProfile {
            secret_id: secret_id.to_string(),
            name: secret_id.to_string(),
            wallet_address: wallet_address.to_string(),
            agent_key: sensitive_string(agent_key).into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }
    }

    fn terminal_with_encrypted_storage(accounts: Vec<AccountProfile>) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.desktop_notifications = false;
        terminal.accounts = accounts;
        terminal.active_account_index = 0;
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

    fn open_window(terminal: &mut TradingTerminal) {
        let _task = terminal.open_add_account_window();
        assert!(terminal.add_account_window.is_some());
    }

    fn window_state(terminal: &mut TradingTerminal) -> &mut AddAccountWindowState {
        terminal
            .add_account_window
            .as_mut()
            .expect("add-account window should be open")
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

    #[test]
    fn open_closes_picker_menu_and_reuses_the_existing_window() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        terminal.account_picker_open = true;
        terminal.account_picker_rename_index = Some(0);

        open_window(&mut terminal);

        assert!(!terminal.account_picker_open);
        assert_eq!(terminal.account_picker_rename_index, None);
        let first_window_id = window_state(&mut terminal).window_id;

        let _task = terminal.open_add_account_window();
        assert_eq!(window_state(&mut terminal).window_id, first_window_id);
    }

    #[test]
    fn submit_rejects_invalid_address_without_creating_a_profile() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        open_window(&mut terminal);
        window_state(&mut terminal).address_input = "0x1234".to_string();

        let _task = terminal.submit_add_account();

        assert_eq!(terminal.accounts.len(), 1);
        let state = window_state(&mut terminal);
        let error = state
            .error
            .as_deref()
            .expect("invalid address should error");
        assert!(error.contains("master account address"));
    }

    #[test]
    fn submit_rejects_invalid_agent_key_without_creating_a_profile() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string("not-a-key");
        }

        let _task = terminal.submit_add_account();

        assert_eq!(terminal.accounts.len(), 1);
        let state = window_state(&mut terminal);
        let error = state.error.as_deref().expect("invalid key should error");
        assert!(error.contains("Agent key cannot be used for trading"));
    }

    #[test]
    fn submit_adds_watch_only_profile_and_switches_to_it() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        terminal.last_persisted_active_account_secret_id = Some("acct-a".to_string());
        open_window(&mut terminal);
        window_state(&mut terminal).address_input = ADDRESS_B.to_string();

        let _task = terminal.submit_add_account_with(|_, _| {
            panic!("watch-only add must not invoke credential persistence")
        });

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.accounts.len(), 2);
        assert_eq!(terminal.accounts[1].wallet_address, ADDRESS_B);
        assert!(terminal.accounts[1].agent_key.trim().is_empty());
        assert_eq!(terminal.accounts[1].name, "Account 2");
        assert_eq!(terminal.active_account_index, 1);
        assert_eq!(terminal.wallet_address_input, ADDRESS_B);
        assert_eq!(
            terminal.active_account_source,
            ActiveAccountSource::Hyperliquid
        );
    }

    #[test]
    fn submit_with_key_persists_to_the_encrypted_store() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.name_input = "Fresh Wallet".to_string();
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(VALID_KEY);
        }

        let _task = terminal.submit_add_account_with(|terminal, persisted_accounts| {
            assert_eq!(terminal.accounts.len(), 2);
            assert!(terminal.accounts[1].agent_key.is_empty());
            assert_eq!(persisted_accounts[1].agent_key.as_str(), VALID_KEY);
            terminal.persist_profile_secrets_from_accounts(persisted_accounts)
        });

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.accounts.len(), 2);
        assert_eq!(terminal.accounts[1].name, "Fresh Wallet");
        assert_eq!(terminal.accounts[1].agent_key.as_str(), VALID_KEY);
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should be rewritten"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(
            payload.profile_agent_key_for_wallet(&terminal.accounts[1].secret_id, ADDRESS_B),
            Some(VALID_KEY)
        );
        assert!(!terminal.secret_migration_save_blocked);
    }

    #[test]
    fn submit_with_locked_encrypted_store_fails_without_creating_a_profile() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        terminal.encrypted_secrets_unlocked = false;
        let original_encrypted = terminal.encrypted_secrets.clone();
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(VALID_KEY);
        }
        let draft_key_allocation = window_state(&mut terminal).key_input.as_str().as_ptr();

        let _task = terminal.submit_add_account_with(|terminal, persisted_accounts| {
            assert_eq!(terminal.accounts.len(), 2);
            assert!(terminal.accounts[1].agent_key.is_empty());
            assert_eq!(persisted_accounts[1].agent_key.as_str(), VALID_KEY);
            terminal.persist_profile_secrets_from_accounts(persisted_accounts)
        });

        assert_eq!(terminal.accounts.len(), 1);
        assert_eq!(terminal.encrypted_secrets, original_encrypted);
        assert!(!terminal.secret_migration_save_blocked);
        let state = window_state(&mut terminal);
        assert_eq!(state.key_input.as_str().as_ptr(), draft_key_allocation);
        let error = state.error.as_deref().expect("locked store should error");
        assert!(error.contains("Unlock encrypted credentials"));
        assert!(error.contains("The account was not added"));
    }

    #[test]
    fn keychain_failure_keeps_exact_draft_and_rolls_back_unsigned_profile_shell() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(VALID_KEY);
        }
        let draft_key_allocation = window_state(&mut terminal).key_input.as_str().as_ptr();
        let staged_key_allocation = Cell::new(std::ptr::null::<u8>());
        let persistence_calls = Cell::new(0_u32);

        let _task = terminal.submit_add_account_with(|terminal, persisted_accounts| {
            persistence_calls.set(persistence_calls.get().saturating_add(1));
            assert_eq!(terminal.accounts.len(), 2);
            assert!(terminal.accounts[1].agent_key.is_empty());
            assert_eq!(persisted_accounts.len(), 2);
            assert_eq!(persisted_accounts[1].agent_key.as_str(), VALID_KEY);
            assert_eq!(
                terminal.accounts[1].secret_id,
                persisted_accounts[1].secret_id
            );
            staged_key_allocation.set(persisted_accounts[1].agent_key.as_ptr());
            terminal.secret_migration_save_blocked = true;
            terminal.secret_store_status = Some((
                "Keychain save failed; credentials were not committed".to_string(),
                true,
            ));
            false
        });

        assert_eq!(persistence_calls.get(), 1);
        assert!(!staged_key_allocation.get().is_null());
        assert_eq!(terminal.accounts.len(), 1);
        assert!(!terminal.secret_migration_save_blocked);
        let state = window_state(&mut terminal);
        assert_eq!(state.key_input.as_str(), VALID_KEY);
        assert_eq!(state.key_input.as_str().as_ptr(), draft_key_allocation);
        let error = state
            .error
            .as_deref()
            .expect("keychain failure should error");
        assert!(error.contains("Keychain save failed"));
        assert!(error.contains("The account was not added"));
    }

    #[test]
    fn first_account_success_moves_staged_and_draft_key_allocations_to_final_owners() {
        let mut terminal = terminal_with_encrypted_storage(Vec::new());
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(VALID_KEY);
        }
        let draft_key_allocation = window_state(&mut terminal).key_input.as_str().as_ptr();
        let staged_key_allocation = Cell::new(std::ptr::null::<u8>());

        let _task = terminal.submit_add_account_with(|terminal, persisted_accounts| {
            assert_eq!(terminal.accounts.len(), 1);
            assert!(terminal.accounts[0].agent_key.is_empty());
            assert_eq!(persisted_accounts.len(), 1);
            assert_eq!(persisted_accounts[0].agent_key.as_str(), VALID_KEY);
            assert_eq!(
                terminal.accounts[0].secret_id,
                persisted_accounts[0].secret_id
            );
            staged_key_allocation.set(persisted_accounts[0].agent_key.as_ptr());
            true
        });

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.accounts.len(), 1);
        assert_eq!(terminal.accounts[0].agent_key.as_str(), VALID_KEY);
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            staged_key_allocation.get()
        );
        assert_eq!(terminal.wallet_key_input.as_str(), VALID_KEY);
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            draft_key_allocation
        );
        assert!(terminal.account_connect_pending);
    }

    #[test]
    fn storage_callback_cannot_retarget_first_account_origin_or_key() {
        let mut terminal = terminal_with_encrypted_storage(Vec::new());
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(VALID_KEY);
        }
        let staged_key_allocation = Cell::new(std::ptr::null::<u8>());
        let changed_draft_allocation = Cell::new(std::ptr::null::<u8>());

        let _task = terminal.submit_add_account_with(|terminal, persisted_accounts| {
            staged_key_allocation.set(persisted_accounts[0].agent_key.as_ptr());
            terminal.accounts[0].name = "Changed by storage callback".to_string();
            terminal.accounts[0].wallet_address = ADDRESS_A.to_string();
            let state = terminal
                .add_account_window
                .as_mut()
                .expect("storage callback should retain the draft window");
            state.key_input = sensitive_string(OTHER_VALID_KEY);
            changed_draft_allocation.set(state.key_input.as_str().as_ptr());
            true
        });

        assert_eq!(terminal.accounts[0].agent_key.as_str(), VALID_KEY);
        assert_eq!(terminal.accounts[0].name, "Account 1");
        assert_eq!(terminal.accounts[0].wallet_address, ADDRESS_B);
        assert_eq!(terminal.wallet_address_input, ADDRESS_B);
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            staged_key_allocation.get()
        );
        assert_eq!(terminal.wallet_key_input.as_str(), VALID_KEY);
        assert_ne!(
            terminal.wallet_key_input.as_str().as_ptr(),
            changed_draft_allocation.get()
        );
    }

    #[test]
    fn switch_on_add_preserves_authorized_target_and_active_input_values() {
        let mut terminal =
            terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "old-agent-key")]);
        terminal.wallet_address_input = ADDRESS_A.to_string();
        terminal.wallet_key_input = sensitive_string("old-agent-key");
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(VALID_KEY);
        }
        let draft_key_allocation = window_state(&mut terminal).key_input.as_str().as_ptr();
        let staged_key_allocation = Cell::new(std::ptr::null::<u8>());

        let _task = terminal.submit_add_account_with(|terminal, persisted_accounts| {
            assert_eq!(terminal.accounts.len(), 2);
            assert!(terminal.accounts[1].agent_key.is_empty());
            assert_eq!(persisted_accounts.len(), 2);
            staged_key_allocation.set(persisted_accounts[1].agent_key.as_ptr());
            true
        });

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.active_account_index, 1);
        assert_eq!(terminal.accounts[1].agent_key.as_str(), VALID_KEY);
        assert_eq!(
            terminal.accounts[1].agent_key.as_ptr(),
            staged_key_allocation.get()
        );
        assert_eq!(terminal.wallet_key_input.as_str(), VALID_KEY);
        assert_ne!(
            terminal.accounts[1].agent_key.as_ptr(),
            draft_key_allocation
        );
        assert_eq!(
            terminal.active_account_source,
            ActiveAccountSource::Hyperliquid
        );
    }

    #[test]
    fn blocked_switch_on_add_never_retargets_the_active_key_input() {
        let mut terminal =
            terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "old-agent-key")]);
        terminal.wallet_address_input = ADDRESS_A.to_string();
        terminal.wallet_key_input = sensitive_string("old-agent-key");
        terminal.connected_address = Some(ADDRESS_A.to_string());
        terminal.chase_orders.insert(42, chase_order(ADDRESS_A));
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(VALID_KEY);
        }
        let active_key_allocation = terminal.wallet_key_input.as_str().as_ptr();
        let staged_key_allocation = Cell::new(std::ptr::null::<u8>());

        let _task = terminal.submit_add_account_with(|terminal, persisted_accounts| {
            assert!(terminal.accounts[1].agent_key.is_empty());
            staged_key_allocation.set(persisted_accounts[1].agent_key.as_ptr());
            true
        });

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.accounts.len(), 2);
        assert_eq!(terminal.active_account_index, 0);
        assert_eq!(terminal.accounts[1].agent_key.as_str(), VALID_KEY);
        assert_eq!(
            terminal.accounts[1].agent_key.as_ptr(),
            staged_key_allocation.get()
        );
        assert_eq!(terminal.wallet_key_input.as_str(), "old-agent-key");
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            active_key_allocation
        );
        assert!(terminal.chase_orders.contains_key(&42));
        let blocked_toast = terminal
            .toasts
            .iter()
            .find(|toast| toast.is_error)
            .expect("blocked switch should toast");
        assert!(blocked_toast.message.contains("switching accounts"));
        let added_toast = terminal
            .toasts
            .iter()
            .find(|toast| !toast.is_error)
            .expect("add should still toast");
        assert!(added_toast.message.contains("without switching"));
    }

    #[test]
    fn first_account_success_preserves_trimmed_key_semantics() {
        let mut terminal = terminal_with_encrypted_storage(Vec::new());
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(format!("  {VALID_KEY}\n"));
        }

        let _task = terminal.submit_add_account_with(|_, persisted_accounts| {
            assert_eq!(persisted_accounts[0].agent_key.as_str(), VALID_KEY);
            true
        });

        assert_eq!(terminal.accounts[0].agent_key.as_str(), VALID_KEY);
        assert_eq!(terminal.wallet_key_input.as_str(), VALID_KEY);
    }

    #[test]
    fn submit_without_switch_keeps_the_active_account() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        terminal.wallet_address_input = ADDRESS_A.to_string();
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.switch_on_add = false;
        }

        let _task = terminal.submit_add_account();

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.accounts.len(), 2);
        assert_eq!(terminal.active_account_index, 0);
        assert_eq!(terminal.wallet_address_input, ADDRESS_A);
        let toast = terminal.toasts.last().expect("add should toast");
        assert!(!toast.is_error);
        assert!(toast.message.contains("Added account"));
    }

    #[test]
    fn submit_still_adds_the_account_when_the_switch_is_blocked_by_a_chase() {
        let mut terminal =
            terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "agent-key")]);
        terminal.connected_address = Some(ADDRESS_A.to_string());
        terminal.chase_orders.insert(42, chase_order(ADDRESS_A));
        open_window(&mut terminal);
        window_state(&mut terminal).address_input = ADDRESS_B.to_string();

        let _task = terminal.submit_add_account();

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.accounts.len(), 2);
        assert_eq!(terminal.active_account_index, 0);
        assert!(terminal.chase_orders.contains_key(&42));
        let blocked_toast = terminal
            .toasts
            .iter()
            .find(|toast| toast.is_error)
            .expect("blocked switch should toast");
        assert!(blocked_toast.message.contains("switching accounts"));
        let added_toast = terminal
            .toasts
            .iter()
            .find(|toast| !toast.is_error)
            .expect("add should still toast");
        assert!(added_toast.message.contains("without switching"));
    }

    #[test]
    fn first_account_submit_syncs_the_active_inputs_and_connects() {
        let mut terminal = terminal_with_encrypted_storage(Vec::new());
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.key_input = sensitive_string(VALID_KEY);
        }

        let _task = terminal.submit_add_account();

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.accounts.len(), 1);
        assert_eq!(terminal.active_account_index, 0);
        assert_eq!(terminal.wallet_address_input, ADDRESS_B);
        assert_eq!(terminal.wallet_key_input.as_str(), VALID_KEY);
        assert!(terminal.account_connect_pending);
        assert_eq!(
            terminal.last_persisted_active_account_secret_id.as_deref(),
            Some(terminal.accounts[0].secret_id.as_str())
        );
    }

    #[test]
    fn first_account_submit_without_switch_syncs_inputs_without_connecting() {
        let mut terminal = terminal_with_encrypted_storage(Vec::new());
        open_window(&mut terminal);
        {
            let state = window_state(&mut terminal);
            state.address_input = ADDRESS_B.to_string();
            state.switch_on_add = false;
        }

        let _task = terminal.submit_add_account();

        assert_eq!(terminal.accounts.len(), 1);
        assert_eq!(terminal.wallet_address_input, ADDRESS_B);
        assert!(!terminal.account_connect_pending);
    }

    #[test]
    fn cancel_drops_the_draft_state() {
        let mut terminal = terminal_with_encrypted_storage(vec![account("acct-a", ADDRESS_A, "")]);
        open_window(&mut terminal);
        window_state(&mut terminal).key_input = sensitive_string(VALID_KEY);

        let _task = terminal.cancel_add_account_window();

        assert!(terminal.add_account_window.is_none());
        assert_eq!(terminal.accounts.len(), 1);
    }
}
