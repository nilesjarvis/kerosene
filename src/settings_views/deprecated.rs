use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{button, column, container, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_settings_deprecated_placeholder(&self) -> Element<'_, Message> {
        let theme = self.theme();
        container(
            column![
                text("Settings have moved to a dedicated window.")
                    .size(14)
                    .color(theme.palette().text),
                button(text("Open Settings").size(12))
                    .padding([4, 8])
                    .on_press(Message::OpenSettingsWindow)
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
    }
}
