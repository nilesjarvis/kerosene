use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{Space, button, text, text_input};
use iced::{Element, Theme};

// ---------------------------------------------------------------------------
// Account Summary Profile Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn summary_account_picker(&self) -> Element<'_, Message> {
        let theme = self.theme();
        self.view_account_picker_button(&theme)
    }

    pub(crate) fn summary_account_label_input(&self) -> Element<'_, Message> {
        let account_label_value = self
            .accounts
            .get(self.active_account_index)
            .map(|profile| profile.name.as_str())
            .unwrap_or("");

        text_input("Account label", account_label_value)
            .style(helpers::text_input_style)
            .on_input(Message::AccountLabelChanged)
            .size(11)
            .padding([4, 6])
            .width(iced::Length::Fixed(140.0))
            .into()
    }

    pub(crate) fn summary_add_account_button(&self) -> Element<'_, Message> {
        button(text("+ Account").size(10).center())
            .on_press(Message::AddAccount)
            .padding([2, 8])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().primary,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    pub(crate) fn summary_forget_ghost_button(&self) -> Element<'_, Message> {
        if self.active_account_is_ghost() {
            button(text("Forget Ghost").size(10).center())
                .on_press(Message::ForgetGhostAccount(self.active_account_index))
                .padding([2, 8])
                .style(|theme: &Theme, status| {
                    let bg = match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ => theme.extended_palette().background.weak.color,
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: theme.palette().warning,
                        border: iced::Border {
                            radius: 3.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .into()
        } else {
            Space::new().width(0).into()
        }
    }

    pub(crate) fn summary_delete_account_button(&self) -> Element<'_, Message> {
        if !self.active_account_is_ghost() && self.accounts.get(self.active_account_index).is_some()
        {
            button(text("Delete Account").size(10).center())
                .on_press(Message::DeleteSavedAccount(self.active_account_index))
                .padding([2, 8])
                .style(|theme: &Theme, status| {
                    let bg = match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ => theme.extended_palette().background.weak.color,
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: theme.palette().danger,
                        border: iced::Border {
                            radius: 3.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .into()
        } else {
            Space::new().width(0).into()
        }
    }

    pub(crate) fn summary_secret_status(&self) -> Option<Element<'_, Message>> {
        let theme = self.theme();
        self.secret_store_status.as_ref().map(|(status, is_error)| {
            text(status)
                .size(10)
                .color(if *is_error {
                    theme.palette().danger
                } else {
                    theme.extended_palette().background.weak.text
                })
                .into()
        })
    }
}
