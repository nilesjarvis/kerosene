mod ghost;
mod saved_delete;

#[cfg(test)]
mod tests;

use crate::app_state::TradingTerminal;
use crate::config::{self, AccountProfile};
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::Task;
use zeroize::{Zeroize, Zeroizing};

/// Short-lived target captured only after every account-switch gate passes.
/// Its saved-profile key becomes the key-input allocation by move.
struct AccountSwitchTarget {
    secret_id: String,
    wallet_address: String,
    agent_key: Zeroizing<String>,
}

impl AccountSwitchTarget {
    fn from_profile(profile: &AccountProfile, is_ghost: bool) -> Self {
        Self {
            secret_id: profile.secret_id.clone(),
            wallet_address: profile.wallet_address.clone(),
            agent_key: if is_ghost {
                Zeroizing::new(String::new())
            } else {
                profile.agent_key.clone()
            },
        }
    }
}

impl TradingTerminal {
    fn stop_twaps_for_account_switch(&mut self) {
        let stop_twap_ids: Vec<u64> = self
            .twap_orders
            .iter()
            .filter_map(|(id, twap)| {
                (!twap.status.is_terminal() && !twap.stop_requested).then_some(*id)
            })
            .collect();

        for id in stop_twap_ids {
            let _ = self.stop_twap_with_reason(id, "TWAP stopped: account switched", false);
        }
    }

    fn clear_connected_account_state_for_switch(&mut self) {
        self.invalidate_account_data_requests();
        self.clear_percentage_order_quantity();
        self.bump_account_data_revision();
        self.rotate_account_user_data_stream();
        if let Some(previous_address) = self.connected_address.clone() {
            self.rotate_wallet_detail_user_data_stream_if_open(&previous_address);
        }
        self.connected_address = None;
        self.account_data = None;
        self.account_data_address = None;
        self.pending_order_indicators.clear();
        self.hud_placements.clear();
        self.pending_cancel_status_request = None;
        self.pending_move_status_request = None;
        self.clear_pending_move_order_state();
        self.pending_leverage_update = None;
        self.order_leverage_dropdown_open = false;
        self.account_loading = false;
        self.account_refresh_followup_pending = false;
        self.account_reconciliation_required = false;
        self.account_error = None;
        self.account_refresh_backoff_until_ms = None;
        self.account_refresh_retry_due_ms = None;
        self.clear_portfolio_income_account_state();
        self.clear_account_scoped_chart_state();
        if self.journal.window_id.is_some() {
            self.journal.clear_active_account_data();
        }
        self.sync_all_chart_overlays();
    }

    pub(crate) fn clear_account_scoped_chart_state(&mut self) {
        self.close_menu_coin = None;
        self.nuke_confirmation = None;
        self.pending_nuke_execution = None;
        for instance in self.charts.values_mut() {
            instance.reset_quick_order_for_account_reset();
            instance.chart.clear_account_scoped_hud_state();
            instance.chart.active_position = None;
            instance.chart.active_orders.clear();
            instance.chart.trade_markers.clear();
        }
        self.chart_quick_order_surface.clear();
    }

    fn load_deferred_legacy_account_key(&mut self, index: usize) {
        self.load_deferred_legacy_account_key_with(
            index,
            config::load_legacy_profile_secrets,
            |terminal| terminal.persist_active_profile_secrets(),
        );
    }

    fn load_deferred_legacy_account_key_with(
        &mut self,
        index: usize,
        load_profile_secrets: impl FnOnce(&mut AccountProfile) -> Result<(), String>,
        persist_profile_secrets: impl FnOnce(&mut Self) -> bool,
    ) {
        if self.secret_storage_mode != config::CredentialStorageMode::OsKeychain
            || self.account_index_is_ghost(index)
        {
            return;
        }

        let Some(profile) = self.accounts.get(index) else {
            return;
        };
        if !profile.agent_key.trim().is_empty() {
            return;
        }

        // Legacy keychain lookup needs the profile ID and must preserve any
        // already-loaded per-profile Hydromancer fallback. It does not need a
        // clone of account metadata or the canonical agent-key owner.
        let mut legacy_profile = AccountProfile {
            secret_id: profile.secret_id.clone(),
            name: String::new(),
            wallet_address: String::new(),
            agent_key: String::new().into(),
            hydromancer_api_key: profile.hydromancer_api_key.clone(),
        };
        match load_profile_secrets(&mut legacy_profile) {
            Ok(()) if !legacy_profile.agent_key.trim().is_empty() => {
                let agent_key = std::mem::take(&mut legacy_profile.agent_key);
                let profile_hydromancer_key =
                    std::mem::take(&mut legacy_profile.hydromancer_api_key);
                let migrated_hydromancer_key = match self
                    .merge_deferred_legacy_profile_hydromancer_key(profile_hydromancer_key)
                {
                    Ok(migrated) => migrated,
                    Err(error) => {
                        self.secret_store_status = Some((
                            format!("{error}; legacy account credentials were left unchanged"),
                            true,
                        ));
                        return;
                    }
                };
                let key_input = agent_key.clone();
                if let Some(profile) = self.accounts.get_mut(index) {
                    profile.agent_key.zeroize();
                    profile.agent_key = agent_key;
                }
                self.wallet_key_input.zeroize();
                self.wallet_key_input = key_input.into();
                if persist_profile_secrets(self) {
                    let message = if migrated_hydromancer_key {
                        "Legacy account key and Hydromancer key migrated to the OS keychain bundle"
                    } else {
                        "Legacy account key migrated to the OS keychain bundle"
                    };
                    self.secret_store_status = Some((message.to_string(), false));
                }
            }
            Ok(()) => {}
            Err(error) => {
                let error = redact_sensitive_response_text(&error);
                self.secret_store_status =
                    Some((format!("Legacy account key read failed: {error}"), true));
            }
        }
    }

    fn merge_deferred_legacy_profile_hydromancer_key(
        &mut self,
        profile_hydromancer_key: Zeroizing<String>,
    ) -> Result<bool, String> {
        if profile_hydromancer_key.trim().is_empty() {
            return Ok(false);
        }

        let current_hydromancer_key = self.hydromancer_api_key.trim();
        if current_hydromancer_key == profile_hydromancer_key.trim() {
            return Ok(false);
        }
        if !current_hydromancer_key.is_empty() {
            return Err(
                "Multiple legacy Hydromancer API keys were found; choose and save the intended key before switching accounts"
                    .to_string(),
            );
        }

        let profile_hydromancer_key =
            if profile_hydromancer_key.trim().len() == profile_hydromancer_key.len() {
                profile_hydromancer_key
            } else {
                Zeroizing::new(profile_hydromancer_key.trim().to_string())
            };
        self.hydromancer_api_key.zeroize();
        self.hydromancer_api_key = profile_hydromancer_key.into();
        self.hydromancer_key_input.zeroize();
        self.hydromancer_key_input = self.hydromancer_api_key.clone();
        self.bump_hydromancer_key_generation();
        self.journal.snapshot_requests.clear();
        self.journal.clear_snapshot_cache();
        self.journal.expanded_snapshot_trade_ids.clear();
        Ok(true)
    }

    pub(crate) fn account_index_for_secret_id(&self, secret_id: &str) -> Option<usize> {
        self.accounts
            .iter()
            .position(|profile| profile.secret_id == secret_id)
    }

    pub(crate) fn fallback_persisted_account_index(&self) -> Option<usize> {
        self.last_persisted_active_account_secret_id
            .as_deref()
            .and_then(|secret_id| self.account_index_for_secret_id(secret_id))
            .filter(|index| !self.account_index_is_ghost(*index))
            .or_else(|| {
                self.accounts
                    .iter()
                    .position(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
            })
    }

    pub(crate) fn reset_account_stream_status(&mut self) {
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
    }

    pub(crate) fn account_change_blocked_by_active_chase(&mut self, action: &str) -> bool {
        if !self.has_active_chase_orders() {
            return false;
        }

        self.push_toast(
            format!("Stop active chase orders and wait for cancellation to finish before {action}"),
            true,
        );
        true
    }

    pub(crate) fn account_change_blocked_by_active_twap(&mut self, action: &str) -> bool {
        if !self.has_active_twap_orders() {
            return false;
        }

        self.push_toast(
            format!("Stop active TWAP orders and wait for cancellation to finish before {action}"),
            true,
        );
        true
    }

    pub(crate) fn account_change_blocked_by_uncertain_twap(&mut self, action: &str) -> bool {
        if !self.has_uncertain_twap_exchange_state() {
            return false;
        }

        self.push_toast(
            format!("Wait for TWAP order status and fill reconciliation to finish before {action}"),
            true,
        );
        true
    }

    pub(crate) fn account_change_blocked_by_active_automation(&mut self, action: &str) -> bool {
        self.account_change_blocked_by_active_chase(action)
            || self.account_change_blocked_by_active_twap(action)
    }

    pub(crate) fn has_active_order_automation(&self) -> bool {
        self.has_active_chase_orders() || self.has_active_twap_orders()
    }

    fn has_active_chase_orders(&self) -> bool {
        !self.chase_orders.is_empty()
    }

    fn has_active_twap_orders(&self) -> bool {
        self.twap_orders
            .values()
            .any(|twap| !twap.status.is_terminal())
    }

    fn has_uncertain_twap_exchange_state(&self) -> bool {
        self.twap_orders.values().any(|twap| {
            !twap.status.is_terminal()
                && (twap.pending_op.is_some()
                    || twap.status_check_cloid.is_some()
                    || twap.reconciliation_deadline.is_some()
                    || twap.has_status_unknown_child())
        })
    }

    pub(crate) fn account_change_blocked_by_pending_trading_request(
        &mut self,
        action: &str,
    ) -> bool {
        if !self.has_pending_trading_request() {
            return false;
        }

        self.push_toast(
            format!("Wait for pending trading requests to finish before {action}"),
            true,
        );
        true
    }

    pub(crate) fn switch_account_task(&mut self, index: usize) -> Task<Message> {
        self.switch_account_task_with_target(index, AccountSwitchTarget::from_profile)
    }

    fn switch_account_task_with_target(
        &mut self,
        index: usize,
        capture_target: impl FnOnce(&AccountProfile, bool) -> AccountSwitchTarget,
    ) -> Task<Message> {
        if self.accounts.get(index).is_none() {
            return Task::none();
        }

        if index == self.active_account_index {
            return Task::none();
        }

        if self.account_change_blocked_by_pending_trading_request("switching accounts") {
            return Task::none();
        }

        if self.account_change_blocked_by_active_chase("switching accounts") {
            return Task::none();
        }

        if self.account_change_blocked_by_uncertain_twap("switching accounts") {
            return Task::none();
        }

        self.stop_twaps_for_account_switch();
        self.clear_connected_account_state_for_switch();

        let profile = self
            .accounts
            .get(index)
            .expect("switch target must survive synchronous account-state cleanup");
        let is_ghost = self.ghost_account_secret_ids.contains(&profile.secret_id);
        let target = capture_target(profile, is_ghost);

        self.active_account_index = index;
        self.journal
            .switch_active_account(Some(target.secret_id.clone()));
        self.wallet_address_input = target.wallet_address;
        self.close_menu_coin = None;
        self.nuke_confirmation = None;
        self.pending_nuke_execution = None;
        self.show_hidden_positions = false;
        if is_ghost {
            self.wallet_key_input.zeroize();
            if let Some(profile) = self.accounts.get_mut(index) {
                profile.agent_key.zeroize();
            }
            self.secret_store_status = Some(("Ghost wallet loaded in memory only".into(), false));
        } else {
            self.wallet_key_input.zeroize();
            self.wallet_key_input = target.agent_key.into();
            self.last_persisted_active_account_secret_id = Some(target.secret_id);
            if self.wallet_key_input.trim().is_empty() {
                self.load_deferred_legacy_account_key(index);
            }
        }

        self.reset_account_stream_status();
        self.persist_config();

        if !self.wallet_address_input.trim().is_empty() {
            // Mark the connect as in-flight so the summary renders the connecting
            // skeleton during the gap before ConnectWallet is processed, rather
            // than flashing the disconnected add-account form.
            self.account_connect_pending = true;
            Task::done(Message::ConnectWallet)
        } else {
            self.account_connect_pending = false;
            Task::done(Message::DisconnectWallet)
        }
    }

    pub(crate) fn account_can_trade(profile: &AccountProfile) -> bool {
        !profile.agent_key.trim().is_empty()
    }

    pub(crate) fn active_account_can_trade(&self) -> bool {
        self.accounts
            .get(self.active_account_index)
            .is_some_and(|profile| {
                !self.ghost_account_secret_ids.contains(&profile.secret_id)
                    && Self::account_can_trade(profile)
            })
    }
}
