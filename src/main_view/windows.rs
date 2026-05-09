use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Element, Theme, window};

// ---------------------------------------------------------------------------
// Window Routing
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_window(&self, window_id: window::Id) -> Element<'_, Message> {
        if Some(window_id) == self.wallet_tracker.window_id {
            return self.view_wallet_tracker();
        }
        if self.wallet_detail_windows.contains_key(&window_id) {
            return self.view_wallet_details(window_id);
        }
        if Some(window_id) == self.journal.window_id {
            return self.view_journal();
        }
        if Some(window_id) == self.settings_window_id {
            return self.view_settings();
        }
        self.view_main()
    }

    pub(crate) fn window_title(&self, window_id: window::Id) -> String {
        if Some(window_id) == self.wallet_tracker.window_id {
            "Kerosene Wallet Tracker".to_string()
        } else if let Some(state) = self.wallet_detail_windows.get(&window_id) {
            let display = self.wallet_display(&state.address);
            format!("Kerosene Wallet Details - {}", display.primary)
        } else if Some(window_id) == self.journal.window_id {
            "Kerosene Trading Journal".to_string()
        } else if Some(window_id) == self.settings_window_id {
            "Kerosene Settings".to_string()
        } else {
            "Kerosene Trading Terminal".to_string()
        }
    }

    pub(crate) fn window_theme(state: &Self, _window_id: window::Id) -> Theme {
        state.theme()
    }
}
