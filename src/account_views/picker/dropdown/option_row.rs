use crate::account_state::AccountPickerOption;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers;
use crate::message::Message;

use iced::widget::{Row, button, column, container, row, text, text_input};
use iced::{Element, Fill, Theme};

mod components;

use components::{RENAME_ICON, account_action_button, account_option_row_padding};

const ACCOUNT_OPTION_TEXT_LEFT_PADDING: f32 = 6.0;

impl TradingTerminal {
    pub(super) fn view_account_picker_option_row(
        &self,
        option: &AccountPickerOption,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let is_active = option.index == self.active_account_index;
        let label = Self::truncate_display_text(&Self::account_picker_label(option), 28);
        let address = Self::account_picker_address_line(option);
        let address = if self
            .accounts
            .get(option.index)
            .is_some_and(|profile| profile.master_address.is_some())
        {
            format!("{address} · Subaccount")
        } else {
            address
        };
        let active_marker = if is_active { ">" } else { "" };
        let index = option.index;
        let is_renaming = self.account_picker_rename_index == Some(index);
        let label_value = self
            .accounts
            .get(index)
            .map(|profile| profile.name.as_str())
            .unwrap_or("");

        let account_control: Element<'_, Message> = if is_renaming {
            container(
                row![
                    text(active_marker)
                        .size(11)
                        .color(theme.palette().primary)
                        .width(12),
                    container(
                        text_input("Account label", label_value)
                            .style(helpers::text_input_style)
                            .on_input(move |value| Message::AccountPickerLabelChanged(index, value))
                            .size(11)
                            .padding([6, 8])
                            .width(Fill),
                    )
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: ACCOUNT_OPTION_TEXT_LEFT_PADDING,
                    })
                    .width(Fill),
                    Self::account_mode_tag(option.is_ghost, option.can_trade, theme),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
            .padding(account_option_row_padding())
            .width(Fill)
            .into()
        } else {
            let mut account_row = Row::new()
                .push(
                    text(active_marker)
                        .size(11)
                        .color(theme.palette().primary)
                        .width(12),
                )
                .push(
                    container(
                        column![
                            text(label).size(12).color(theme.palette().text),
                            text(address)
                                .size(10)
                                .color(theme.extended_palette().background.weak.text),
                        ]
                        .spacing(2)
                        .width(Fill),
                    )
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: ACCOUNT_OPTION_TEXT_LEFT_PADDING,
                    })
                    .width(Fill),
                );
            if let Some(hotkey) = self.account_picker_hotkey_display(index) {
                account_row = account_row.push(
                    text(hotkey)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                );
            }
            account_row = account_row.push(Self::account_mode_tag(
                option.is_ghost,
                option.can_trade,
                theme,
            ));

            button(account_row.spacing(10).align_y(iced::Alignment::Center))
                .on_press(Message::AccountPickerSelected(index))
                .padding(account_option_row_padding())
                .width(Fill)
                .style(move |theme: &Theme, status| {
                    helpers::selected_row_button_style(theme, status, is_active)
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

        let mut actions = Row::new().spacing(6);
        if self.subaccount_discovery_master_address(index).is_some() {
            actions = actions.push(account_action_button(
                "Subaccounts",
                Message::DiscoverAccountSubaccounts(index),
                theme.palette().primary,
                false,
            ));
        }
        actions = actions
            .push(account_action_button(
                RENAME_ICON,
                Message::AccountPickerRenameToggled(index),
                theme.palette().primary,
                is_renaming,
            ))
            .push(account_action_button(
                delete_label,
                delete_message,
                delete_color,
                false,
            ));

        container(column![
            account_control,
            container(actions.align_y(iced::Alignment::Center)).align_right(Fill),
        ])
        .padding([3, 0])
        .width(Fill)
        .into()
    }

    fn account_picker_hotkey_display(&self, index: usize) -> Option<String> {
        let secret_id = self.accounts.get(index)?.secret_id.clone();
        let action = config::HotkeyAction::SwitchAccount { secret_id };
        self.hotkeys
            .iter()
            .find(|hotkey| hotkey.action == action)
            .map(Self::hotkey_display)
    }
}
