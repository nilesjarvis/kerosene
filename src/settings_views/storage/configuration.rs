use crate::message::Message;

use iced::widget::{button, column, row, text};
use iced::{Element, Fill, Theme};

pub(super) fn configuration_actions(theme: &Theme) -> Element<'static, Message> {
    column![
        row![
            text("Configuration")
                .size(14)
                .color(theme.palette().text)
                .width(Fill),
            button(
                text("Clear All Configs")
                    .size(12)
                    .color(theme.palette().danger)
            )
            .padding([6, 12])
            .on_press(Message::ClearConfigs),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
        text("Deletes saved settings, layouts, backups, and credential entries. Restart to load defaults.")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
    ]
    .spacing(8)
    .into()
}
