use super::CONNECTED_STATUS_ACTION_BREAKPOINT;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, column, container, responsive, row};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_disconnected_account_summary(&self) -> Element<'_, Message> {
        container(responsive(move |size| {
            self.view_disconnected_account_summary_layout(size.width)
        }))
        .width(Fill)
        .height(Fill)
        .padding([6, 12])
        .center_y(Fill)
        .into()
    }

    fn view_disconnected_account_summary_layout(
        &self,
        available_width: f32,
    ) -> Element<'_, Message> {
        let account_row = row![
            self.summary_account_picker(),
            column(self.summary_secret_status()).width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .width(Fill);
        let actions = row![
            self.summary_market_universe_picker(),
            self.summary_layouts_button(),
            self.summary_widgets_button(),
            self.summary_settings_button()
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        if available_width < CONNECTED_STATUS_ACTION_BREAKPOINT {
            column![
                account_row,
                row![Space::new().width(Fill), actions]
                    .width(Fill)
                    .align_y(iced::Alignment::Center),
            ]
            .spacing(6)
            .width(Fill)
            .into()
        } else {
            row![account_row, actions]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .width(Fill)
                .into()
        }
    }
}
