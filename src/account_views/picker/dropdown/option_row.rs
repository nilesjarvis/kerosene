use crate::account_state::AccountPickerOption;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Color, Element, Fill, Length, Theme};

const RENAME_ICON: &str = "✎";

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
        let is_renaming = self.account_picker_rename_index == Some(index);
        let label_value = self
            .accounts
            .get(index)
            .map(|profile| profile.name.as_str())
            .unwrap_or("");

        let account_control: Element<'_, Message> = if is_renaming {
            row![
                text(active_marker)
                    .size(11)
                    .color(theme.palette().primary)
                    .width(12),
                text_input("Account label", label_value)
                    .style(helpers::text_input_style)
                    .on_input(move |value| Message::AccountPickerLabelChanged(index, value))
                    .size(11)
                    .padding([4, 6])
                    .width(Fill),
                Self::account_mode_tag(option.is_ghost, option.can_trade, theme),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        } else {
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
        };

        let delete_message = if option.is_ghost {
            Message::ForgetGhostAccount(index)
        } else {
            Message::DeleteSavedAccount(index)
        };
        let delete_label = if option.is_ghost { "Forget" } else { "Delete" };
        let delete_color = if option.is_ghost {
            theme.palette().warning
        } else {
            theme.palette().danger
        };

        container(
            row![
                container(account_control).width(Fill),
                account_action_button(
                    RENAME_ICON,
                    Message::AccountPickerRenameToggled(index),
                    theme.palette().primary,
                    is_renaming,
                ),
                account_action_button(delete_label, delete_message, delete_color, false),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding([2, 0])
        .width(Fill)
        .into()
    }
}

fn account_action_button(
    label: &'static str,
    message: Message,
    color: Color,
    active: bool,
) -> Element<'static, Message> {
    button(text(label).size(10).center())
        .on_press(message)
        .padding([6, 6])
        .width(if label == RENAME_ICON {
            Length::Fixed(30.0)
        } else {
            Length::Fixed(56.0)
        })
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: color,
                border: iced::Border {
                    radius: 4.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active { color } else { Color::TRANSPARENT },
                },
                ..Default::default()
            }
        })
        .into()
}
