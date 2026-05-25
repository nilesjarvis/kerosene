use crate::app_state::TradingTerminal;
use crate::feed_views::tracked_trades::layout::{
    COIN_WIDTH, WALLET_COLUMN_WIDTH, WALLET_LABEL_WIDTH,
};
use crate::helpers;
use crate::message::Message;
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, row, text, tooltip};
use iced::{Color, Element, Theme};

impl TradingTerminal {
    pub(super) fn view_tracked_trade_wallet_cell(
        &self,
        address_for_message: String,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let address = address_for_message.trim().to_string();
        if address.is_empty() {
            return text("-")
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text)
                .wrapping(Wrapping::None)
                .width(WALLET_COLUMN_WIDTH)
                .into();
        }

        let display = self.wallet_display(&address);
        let wallet_button = button(
            text(display.primary)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().primary)
                .wrapping(Wrapping::None),
        )
        .on_press(Message::CopyToClipboard(address_for_message.clone()))
        .padding(0)
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .width(WALLET_LABEL_WIDTH);

        let wallet_button: Element<'_, Message> = tooltip(
            wallet_button,
            text(address.clone())
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            iced::widget::tooltip::Position::Top,
        )
        .into();

        let ghost_button: Element<'_, Message> = tooltip(
            button(
                text("G")
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .wrapping(Wrapping::None)
                    .center(),
            )
            .on_press(Message::GhostWallet(address_for_message.clone()))
            .padding([0, 4])
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
                        width: 1.0,
                        color: Color {
                            a: 0.45,
                            ..theme.palette().primary
                        },
                    },
                    ..Default::default()
                }
            }),
            text("Open in ghost mode").size(10),
            iced::widget::tooltip::Position::Top,
        )
        .into();

        let details_button: Element<'_, Message> = tooltip(
            button(
                text("\u{2197}")
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .wrapping(Wrapping::None)
                    .center(),
            )
            .on_press(Message::OpenWalletDetailsWindow(address_for_message))
            .padding([0, 4])
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
                        width: 1.0,
                        color: Color {
                            a: 0.45,
                            ..theme.palette().primary
                        },
                    },
                    ..Default::default()
                }
            }),
            text("Open detachable wallet details").size(10),
            iced::widget::tooltip::Position::Top,
        )
        .into();

        row![wallet_button, details_button, ghost_button]
            .spacing(3)
            .align_y(iced::Alignment::Center)
            .width(WALLET_COLUMN_WIDTH)
            .into()
    }

    pub(super) fn view_tracked_trade_coin_cell(&self, coin: String) -> Element<'_, Message> {
        let theme = self.theme();
        let display_coin = self.display_coin_for_spot_balance(&coin);
        let mut coin_content = row![];
        if let Some(icon) = helpers::symbol_icon(&display_coin, 14, theme.palette().text)
            .or_else(|| helpers::symbol_icon(&coin, 14, theme.palette().text))
        {
            coin_content = coin_content.push(icon).push(Space::new().width(4.0));
        }
        coin_content = coin_content
            .push(
                text(display_coin.clone())
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .wrapping(Wrapping::None),
            )
            .align_y(iced::Alignment::Center);

        let raw_coin = coin.clone();
        let coin_button = button(coin_content)
            .on_press(Message::SymbolSelected(coin))
            .padding(0)
            .width(COIN_WIDTH)
            .style(|theme: &Theme, status| {
                let text_color = match status {
                    button::Status::Hovered => theme.palette().primary,
                    _ => theme.palette().text,
                };
                button::Style {
                    background: None,
                    text_color,
                    ..Default::default()
                }
            });

        if display_coin != raw_coin {
            return tooltip(
                coin_button,
                text(raw_coin)
                    .size(10)
                    .font(crate::app_fonts::monospace_font()),
                iced::widget::tooltip::Position::Top,
            )
            .into();
        }

        coin_button.into()
    }
}
