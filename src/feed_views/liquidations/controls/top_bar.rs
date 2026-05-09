mod actions;
mod connection;
mod toggles;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, row};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_liquidations_top_bar(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();

        row![
            self.view_liquidations_connection_controls(now_ms, &theme),
            Space::new().width(Fill),
            self.view_liquidation_display_toggles(),
            self.view_liquidation_threshold_controls(&theme),
            Space::new().width(8),
            self.view_clear_liquidations_button(),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
