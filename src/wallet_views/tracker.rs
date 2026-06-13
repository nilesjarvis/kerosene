use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{column, container, rule, scrollable};
use iced::{Element, Fill, Theme};

mod controls;
mod rows;

// ---------------------------------------------------------------------------
// Wallet Tracker
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_wallet_tracker(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let now_ms = self.status_bar_now_ms;

        container(
            column![
                self.view_wallet_tracker_header(&theme),
                self.view_wallet_tracker_add_row(),
                rule::horizontal(1),
                Self::view_wallet_tracker_table_header(&theme),
                scrollable(self.view_wallet_tracker_list(now_ms, &theme)).height(Fill)
            ]
            .spacing(10),
        )
        .padding(12)
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            ..Default::default()
        })
        .into()
    }
}
