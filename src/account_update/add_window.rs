use crate::account_state::{
    AddAccountTarget, AddAccountWindowState, SubaccountDiscoveryRequest, SubaccountDiscoveryResult,
    fetch_subaccounts,
};
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
    height: 670.0,
};
const ADD_ACCOUNT_WINDOW_MIN_SIZE: Size = Size {
    width: 420.0,
    height: 460.0,
};

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
            ..crate::window_chrome::settings(
                self.custom_window_chrome_active,
                self.window_background_blur_enabled,
            )
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
            if Self::normalize_wallet_address(&state.address_input)
                != Self::normalize_wallet_address(&value)
            {
                state.invalidate_subaccounts();
            }
            state.address_input = value;
            state.error = None;
        }
        Task::none()
    }

    pub(super) fn discover_add_account_subaccounts(&mut self) -> Task<Message> {
        let Some(state) = self.add_account_window.as_mut() else {
            return Task::none();
        };
        state.invalidate_subaccounts();
        state.error = None;
        let Some(master_address) = Self::normalize_wallet_address(&state.address_input) else {
            state.discovery_error = Some("Enter a valid master account address first.".to_string());
            return Task::none();
        };
        state.discovery_generation = state.discovery_generation.wrapping_add(1);
        let request = SubaccountDiscoveryRequest {
            window_id: state.window_id,
            generation: state.discovery_generation,
            master_address: master_address.into(),
        };
        state.discovery_request = Some(request.clone());
        Task::perform(
            async move {
                let result = fetch_subaccounts(&request.master_address).await;
                (request, result)
            },
            |(request, result)| Message::AddAccountSubaccountsLoaded(request, result),
        )
    }

    pub(super) fn apply_add_account_subaccounts(
        &mut self,
        request: SubaccountDiscoveryRequest,
        result: SubaccountDiscoveryResult,
    ) -> Task<Message> {
        let Some(state) = self.add_account_window.as_mut() else {
            return Task::none();
        };
        if state.window_id != request.window_id
            || state.discovery_request.as_ref() != Some(&request)
            || Self::normalize_wallet_address(&state.address_input).as_deref()
                != Some(request.master_address.as_str())
        {
            return Task::none();
        }
        state.discovery_request = None;
        match result.0 {
            Ok(subaccounts) => {
                state.subaccounts = subaccounts;
                state.discovered_master = Some(request.master_address.into_string());
                state.discovery_error = None;
            }
            Err(error) => state.discovery_error = Some(redact_sensitive_response_text(&error)),
        }
        Task::none()
    }

    pub(super) fn select_add_account_target(&mut self, target: AddAccountTarget) -> Task<Message> {
        if let Some(state) = self.add_account_window.as_mut() {
            state.target = Some(target);
            if let Err(error) = state.selected_addresses() {
                state.target = None;
                state.error = Some(error);
            } else {
                state.error = None;
            }
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
        let (window_id, switch_on_add, name, addresses, agent_key) = {
            let Some(state) = self.add_account_window.as_ref() else {
                return Task::none();
            };
            (
                state.window_id,
                state.switch_on_add,
                state.profile_name(self.persisted_accounts_snapshot().len() + 1),
                state.selected_addresses(),
                Zeroizing::new(state.key_input.trim().to_string()),
            )
        };

        let (address, master_address) = match addresses {
            Ok(addresses) => addresses,
            Err(error) => {
                self.set_add_account_error(error);
                return Task::none();
            }
        };

        if !agent_key.is_empty()
            && let Err(error) = signing::validate_agent_key(&agent_key)
        {
            self.set_add_account_error(format!("Agent key cannot be used for trading: {error}"));
            return Task::none();
        }

        let profile = config::AccountProfile {
            master_address,
            secret_id: config::new_secret_id(),
            name,
            wallet_address: address,
            agent_key: agent_key.clone(),
            hydromancer_api_key: String::new().into(),
        };
        let profile_name = profile.name.clone();
        let profile_secret_id = profile.secret_id.clone();

        // Commit the profile, then persist secrets from the committed
        // snapshot; roll the push back if credential storage refuses so an
        // account whose key was never saved cannot appear.
        self.accounts.push(profile);
        let new_index = self.accounts.len() - 1;

        if !agent_key.is_empty() {
            let persisted_accounts = self.persisted_accounts_snapshot();
            let migration_blocked_before = self.secret_migration_save_blocked;
            if !self.persist_profile_secrets_from_accounts(&persisted_accounts, &profile_secret_id)
            {
                self.accounts.pop();
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
        }

        self.persist_config();
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
            self.wallet_key_input = profile.agent_key.clone().into();
            self.last_persisted_active_account_secret_id = Some(secret_id.clone());
            self.journal.switch_active_account(Some(secret_id));
            if switch_on_add {
                self.account_connect_pending = true;
                return Task::batch([close_task, Task::done(Message::ConnectWallet)]);
            }
            self.push_toast(format!("Added account \"{profile_name}\""), false);
            return close_task;
        }

        if switch_on_add {
            let switch_task = self.switch_account_task(new_index);
            if self.active_account_index != new_index {
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

    fn set_add_account_error(&mut self, message: impl Into<String>) {
        if let Some(state) = self.add_account_window.as_mut() {
            state.error = Some(message.into());
        }
    }
}

#[cfg(test)]
mod tests;
