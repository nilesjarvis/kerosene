use crate::message::Message;

use iced::widget::{button, column, row, text};
use iced::{Element, Fill, Theme};

pub(super) fn configuration_actions(
    theme: &Theme,
    clear_block_reason: Option<String>,
) -> Element<'static, Message> {
    let clear_blocked = clear_block_reason.is_some();
    let clear_button = {
        let button = button(
            text("Clear All Configs")
                .size(12)
                .color(theme.palette().danger),
        )
        .padding([6, 12]);

        if clear_blocked {
            button
        } else {
            button.on_press(Message::ClearConfigs)
        }
    };

    let help_text = clear_block_reason.unwrap_or_else(|| {
        "Deletes saved settings, layouts, backups, and credential entries. Restart to load defaults."
            .to_string()
    });

    column![
        row![
            text("Configuration")
                .size(14)
                .color(theme.palette().text)
                .width(Fill),
            clear_button,
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
        text(help_text).size(11).color(if clear_blocked {
            theme.palette().danger
        } else {
            theme.extended_palette().background.weak.text
        }),
    ]
    .spacing(8)
    .into()
}
