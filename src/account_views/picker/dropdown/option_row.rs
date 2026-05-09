use crate::account_state::AccountPickerOption;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{button, column, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_account_picker_option_row(
        &self,
        option: &AccountPickerOption,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let is_active = option.index == self.active_account_index;
        let label = Self::truncate_display_text(&Self::account_picker_label(option), 28);
        let address = Self::account_picker_address_line(option);
        let active_marker = if is_active { ">" } else { "" };
        let index = option.index;

        button(
            row![
                text(active_marker)
                    .size(11)
                    .color(theme.palette().primary)
                    .width(12),
                column![
                    text(label).size(12).color(theme.palette().text),
                    text(address)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(1)
                .width(Fill),
                Self::account_mode_tag(option.is_ghost, option.can_trade, theme),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::AccountPickerSelected(index))
        .padding([7, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if is_active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    width: if is_active { 1.0 } else { 0.0 },
                    color: if is_active {
                        theme.palette().primary
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
    }
}
