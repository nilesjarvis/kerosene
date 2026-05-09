use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::wallet_state::WalletTrackerRow;

use iced::widget::{row, text, tooltip};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_wallet_tracker_state<'a>(
        &'a self,
        row_data: &WalletTrackerRow,
        theme: &Theme,
    ) -> Element<'a, Message> {
        if row_data.loading {
            row![
                self.view_spinner(14),
                text("Refreshing")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into()
        } else if let Some(err) = row_data.error.as_ref() {
            tooltip(
                text("Error").size(11).color(theme.palette().danger),
                text(err.clone()).size(10),
                iced::widget::tooltip::Position::Top,
            )
            .into()
        } else if let Some(updated_at) = row_data.last_updated_ms {
            text(helpers::format_relative_time(updated_at, Self::now_ms()))
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into()
        } else {
            text("Not loaded")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into()
        }
    }
}
