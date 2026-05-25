use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::super::metrics::position_for_coin;
use super::super::{PnlCardDisplayMode, PnlCardPercentMode, PnlCardTarget, PnlCardWindowState};

use iced::{Size, Task, window};

// ---------------------------------------------------------------------------
// PnL Card Window Lifecycle
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn open_pnl_card_window(&mut self, target: PnlCardTarget) -> Task<Message> {
        let Some(account_address) = self.current_pnl_card_account_address() else {
            self.push_toast(
                "Connect an account before opening a PnL card".to_string(),
                true,
            );
            return Task::none();
        };

        if let Some(window_id) = self.pnl_card_windows.iter().find_map(|(id, state)| {
            (state.target == target && state.account_address == account_address).then_some(*id)
        }) {
            return window::gain_focus(window_id);
        }

        if !self.pnl_card_target_available(&target) {
            return Task::none();
        }

        let settings = window::Settings {
            size: Size::new(480.0, 640.0),
            ..crate::window_chrome::settings()
        };
        let (window_id, task) = window::open(settings);
        self.pnl_card_windows
            .insert(window_id, PnlCardWindowState::new(target, account_address));

        task.map(Message::WindowOpened)
    }

    pub(crate) fn set_pnl_card_display_mode(
        &mut self,
        window_id: window::Id,
        mode: PnlCardDisplayMode,
    ) -> Task<Message> {
        if let Some(state) = self.pnl_card_windows.get_mut(&window_id) {
            state.display_mode = mode;
        }
        Task::none()
    }

    pub(crate) fn set_pnl_card_percent_mode(
        &mut self,
        window_id: window::Id,
        mode: PnlCardPercentMode,
    ) -> Task<Message> {
        if let Some(state) = self.pnl_card_windows.get_mut(&window_id) {
            state.percent_mode = mode;
        }
        Task::none()
    }

    pub(crate) fn toggle_pnl_card_price_privacy(
        &mut self,
        window_id: window::Id,
        obscure: bool,
    ) -> Task<Message> {
        if let Some(state) = self.pnl_card_windows.get_mut(&window_id) {
            state.obscure_prices = obscure;
        }
        Task::none()
    }

    pub(crate) fn toggle_pnl_card_position_size(
        &mut self,
        window_id: window::Id,
        show: bool,
    ) -> Task<Message> {
        if let Some(state) = self.pnl_card_windows.get_mut(&window_id) {
            state.show_position_size = show;
        }
        Task::none()
    }

    fn pnl_card_target_available(&self, target: &PnlCardTarget) -> bool {
        match target {
            PnlCardTarget::Position(coin) => self
                .account_data
                .as_ref()
                .is_some_and(|data| position_for_coin(data, coin).is_some()),
            PnlCardTarget::Summary => self.visible_pnl_card_positions().next().is_some(),
        }
    }

    fn current_pnl_card_account_address(&self) -> Option<String> {
        self.connected_address
            .as_deref()
            .and_then(Self::normalize_wallet_address)
    }
}
