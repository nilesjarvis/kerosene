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
        if self
            .twap_orders
            .values()
            .any(|twap| twap.window_id == Some(window_id))
        {
            return self.view_twap_details(window_id);
        }
        if self.advanced_order_history_windows.contains_key(&window_id) {
            return self.view_advanced_order_history_details(window_id);
        }
        if Some(window_id) == self.journal.window_id {
            return self.view_journal();
        }
        if Some(window_id) == self.settings_window_id {
            return self.view_settings();
        }
        if Some(window_id) == self.chart_screenshot_window_id {
            return self.view_chart_screenshot_window();
        }
        if self.pnl_card_windows.contains_key(&window_id) {
            return self.view_pnl_card_window(window_id);
        }
        self.view_main()
    }

    pub(crate) fn window_title(&self, window_id: window::Id) -> String {
        if Some(window_id) == self.wallet_tracker.window_id {
            "Kerosene Wallet Tracker".to_string()
        } else if let Some(state) = self.wallet_detail_windows.get(&window_id) {
            let display = self.wallet_display(&state.address);
            format!("Kerosene Wallet Details - {}", display.primary)
        } else if let Some(twap) = self
            .twap_orders
            .values()
            .find(|twap| twap.window_id == Some(window_id))
        {
            format!("Kerosene TWAP #{} - {}", twap.id, twap.display_coin)
        } else if let Some(entry_id) = self.advanced_order_history_windows.get(&window_id) {
            self.advanced_order_history
                .iter()
                .find(|entry| entry.id == *entry_id)
                .map(|entry| {
                    format!(
                        "Kerosene {} History - {}",
                        entry.kind.label(),
                        entry.display_coin
                    )
                })
                .unwrap_or_else(|| "Kerosene Advanced Order History".to_string())
        } else if Some(window_id) == self.journal.window_id {
            "Kerosene Trading Journal".to_string()
        } else if Some(window_id) == self.settings_window_id {
            "Kerosene Settings".to_string()
        } else if Some(window_id) == self.chart_screenshot_window_id {
            "Kerosene Chart Screenshot".to_string()
        } else if let Some(state) = self.pnl_card_windows.get(&window_id) {
            match &state.target {
                crate::pnl_card::PnlCardTarget::Position(coin) => {
                    format!("Kerosene PnL Card - {coin}")
                }
                crate::pnl_card::PnlCardTarget::Summary => {
                    "Kerosene PnL Card - Summary".to_string()
                }
            }
        } else {
            "Kerosene Trading Terminal".to_string()
        }
    }

    pub(crate) fn window_theme(state: &Self, _window_id: window::Id) -> Theme {
        state.theme()
    }
}
