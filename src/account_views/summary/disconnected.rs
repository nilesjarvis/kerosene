use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_disconnected_account_summary(&self) -> Element<'_, Message> {
        let active_account_is_ghost = self.active_account_is_ghost();

        let key_input: Element<'_, Message> = if active_account_is_ghost {
            text_input("Ghost wallet is watch-only", "")
                .style(helpers::text_input_style)
                .size(11)
                .padding([4, 6])
                .width(Fill)
                .into()
        } else {
            text_input(
                "Agent private key (enables trading)",
                &self.wallet_key_input,
            )
            .style(helpers::text_input_style)
            .on_input(Message::WalletKeyInputChanged)
            .size(11)
            .padding([4, 6])
            .secure(true)
            .width(Fill)
            .into()
        };

        let addr_input = text_input("Master account address (0x...)", &self.wallet_address_input)
            .style(helpers::text_input_style)
            .on_input(Message::WalletAddressInputChanged)
            .size(11)
            .padding([4, 6])
            .width(Fill);

        let connect_btn = button(text("Connect").size(11).center().width(Fill))
            .on_press(Message::ConnectWallet)
            .padding([4, 12])
            .style(|theme: &Theme, _status| button::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let save_credentials_btn = button(text("Save Trading Key").size(10).center())
            .on_press(Message::SaveCredentials)
            .padding([4, 10])
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
            });

        let account_row = row![
            self.summary_account_picker(),
            self.summary_add_account_button(),
            self.summary_account_label_input(),
            self.summary_forget_ghost_button(),
            addr_input
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        let action_row = row![
            key_input,
            connect_btn,
            save_credentials_btn,
            Space::new().width(Fill),
            self.summary_widgets_button(),
            self.summary_settings_button()
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let mut content = column![account_row, action_row].spacing(4);
        if let Some(status) = self.summary_secret_status() {
            content = content.push(status);
        }

        container(content)
            .width(Fill)
            .height(Fill)
            .padding([2, 12])
            .center_y(Fill)
            .into()
    }
}
