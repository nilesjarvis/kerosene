use crate::account_state::AccountPickerOption;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{button, column, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_account_picker_button(&self, theme: &Theme) -> Element<'_, Message> {
        let selected = self
            .account_picker_options()
            .into_iter()
            .find(|option| option.index == self.active_account_index)
            .unwrap_or(AccountPickerOption {
                index: self.active_account_index,
                label: "No account".to_string(),
                address: String::new(),
                can_trade: false,
                is_ghost: false,
            });

        let label = Self::truncate_display_text(&Self::account_picker_label(&selected), 20);
        let address = Self::account_picker_address_line(&selected);
        let arrow = if self.account_picker_open { "^" } else { "v" };

        button(
            row![
                column![
                    text(label).size(12).color(theme.palette().text),
                    text(address)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(1)
                .width(Fill),
                Self::account_mode_tag(selected.is_ghost, selected.can_trade, theme),
                text(arrow)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ToggleAccountPicker)
        .padding([4, 8])
        .width(iced::Length::Fixed(250.0))
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.strong.color,
                },
                ..Default::default()
            }
        })
        .into()
    }
}
