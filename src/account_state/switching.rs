mod ghost;
mod saved_delete;

#[cfg(test)]
mod tests;

use crate::app_state::TradingTerminal;
use crate::config::AccountProfile;
use crate::message::Message;

use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
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
        if self.chase_orders.is_empty() {
            return false;
        }

        self.push_toast(
            format!("Stop active chase orders and wait for cancellation to finish before {action}"),
            true,
        );
        true
    }

    pub(crate) fn switch_account_task(&mut self, index: usize) -> Task<Message> {
        let Some(profile) = self.accounts.get(index).cloned() else {
            return Task::none();
        };

        if index == self.active_account_index {
            return Task::none();
        }

        if self.pending_order_action.is_some() {
            self.push_toast(
                "Wait for the pending order request before switching accounts".to_string(),
                true,
            );
            return Task::none();
        }

        if self.account_change_blocked_by_active_chase("switching accounts") {
            return Task::none();
        }

        let is_ghost = self.ghost_account_secret_ids.contains(&profile.secret_id);
        self.active_account_index = index;
        self.journal
            .switch_active_account(Some(profile.secret_id.clone()));
        self.wallet_address_input = profile.wallet_address.clone();
        self.close_menu_coin = None;
        self.nuke_confirmation = None;
        self.show_hidden_positions = false;
        for inst in self.charts.values_mut() {
            inst.clear_quick_order();
        }
        if is_ghost {
            self.wallet_key_input.zeroize();
            if let Some(profile) = self.accounts.get_mut(index) {
                profile.agent_key.zeroize();
            }
            self.secret_store_status = Some(("Ghost wallet loaded in memory only".into(), false));
        } else {
            self.wallet_key_input.zeroize();
            self.wallet_key_input = profile.agent_key.clone();
            self.last_persisted_active_account_secret_id = Some(profile.secret_id.clone());
        }

        self.reset_account_stream_status();
        self.persist_config();

        if !self.wallet_address_input.trim().is_empty() {
            Task::done(Message::ConnectWallet)
        } else {
            Task::done(Message::DisconnectWallet)
        }
    }

    pub(crate) fn find_account_by_wallet_address(&self, address: &str) -> Option<usize> {
        self.accounts.iter().position(|profile| {
            Self::normalize_wallet_address(&profile.wallet_address)
                .is_some_and(|profile_address| profile_address == address)
        })
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
