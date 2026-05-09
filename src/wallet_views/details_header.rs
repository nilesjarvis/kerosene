use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::wallet_state::WalletDetailsWindowState;
use iced::widget::{Space, button, column, row, text};
use iced::{Element, Fill, Theme, window};

// ---------------------------------------------------------------------------
// Wallet Details Header
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_wallet_details_header<'a>(
        &'a self,
        window_id: window::Id,
        state: &'a WalletDetailsWindowState,
        now_ms: u64,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let display = self.wallet_display(&state.address);
        let status_color = if state.error.is_some() {
            theme.palette().danger
        } else if state.loading {
            theme.palette().primary
        } else {
            theme.palette().success
        };
        let status_text = if state.loading {
            "Refreshing".to_string()
        } else if state.error.is_some() {
            "Error".to_string()
        } else if let Some(updated_at) = state.last_refresh_ms {
            format!("Live {}", helpers::format_relative_time(updated_at, now_ms))
        } else {
            "Waiting".to_string()
        };

        let refresh_button = if state.loading {
            button(text("Refresh").size(10)).padding([3, 8])
        } else {
            button(text("Refresh").size(10))
                .on_press(Message::RefreshWalletDetails(window_id))
                .padding([3, 8])
        };

        row![
            column![
                text("Wallet Details").size(16).color(theme.palette().text),
                row![
                    text(display.primary)
                        .size(12)
                        .font(iced::Font::MONOSPACE)
                        .color(theme.palette().primary),
                    text(display.secondary)
                        .size(11)
                        .font(iced::Font::MONOSPACE)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(3),
            Space::new().width(Fill),
            row![
                if state.loading {
                    self.view_spinner(14)
                } else {
                    Space::new().width(14.0).height(14.0).into()
                },
                text(status_text)
                    .size(11)
                    .font(iced::Font::MONOSPACE)
                    .color(status_color),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
            button(text("Copy").size(10))
                .on_press(Message::CopyToClipboard(state.address.clone()))
                .padding([3, 8]),
            button(text("Ghost").size(10))
                .on_press(Message::GhostWallet(state.address.clone()))
                .padding([3, 8]),
            refresh_button,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
