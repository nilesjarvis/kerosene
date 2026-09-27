use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{button, column, row, text, text_input};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_remote_wallet_database_settings(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let remote = &self.wallet_tracker.remote_database;
        let configured = !remote.url.is_empty();
        let status = remote
            .input_error
            .clone()
            .unwrap_or_else(|| remote.status_text());
        column![
            text("Remote wallet database").size(14),
            row![
                text_input("http://127.0.0.1:8090", &remote.input)
                    .style(helpers::text_input_style)
                    .on_input(|value| Message::RemoteWalletDatabaseUrlChanged(value.into()))
                    .on_submit(Message::SaveRemoteWalletDatabase)
                    .size(12)
                    .padding(6)
                    .width(Fill),
                button(text("Save").size(12))
                    .padding([6, 12])
                    .on_press(Message::SaveRemoteWalletDatabase),
                button(text("Sync now").size(12))
                    .padding([6, 12])
                    .on_press_maybe(
                        (configured && remote.pending_request.is_none())
                            .then_some(Message::RemoteWalletDatabaseSync)
                    ),
                button(text("Disconnect").size(12))
                    .padding([6, 12])
                    .on_press_maybe(configured.then_some(Message::DisconnectRemoteWalletDatabase)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            text(status).size(11).color(
                if remote.error.is_some() || remote.input_error.is_some() {
                    theme.palette().danger
                } else {
                    theme.extended_palette().background.weak.text
                }
            ),
            text("PocketBase · read-only · manage wallets and labels in your database")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .into()
    }
}
