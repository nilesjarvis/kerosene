use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{Space, button, row, text, tooltip};
use iced::{Color, Element, Theme};

const WALLET_COLUMN_WIDTH: u32 = 164;
const WALLET_LABEL_WIDTH: u32 = 108;

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
                .font(iced::Font::MONOSPACE)
                .color(theme.extended_palette().background.weak.text)
                .width(WALLET_COLUMN_WIDTH)
                .into();
        }

        let display = self.wallet_display(&address);
        let wallet_button = button(
            text(display.primary)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(theme.palette().primary),
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
            text(address.clone()).size(10).font(iced::Font::MONOSPACE),
            iced::widget::tooltip::Position::Top,
        )
        .into();

        let ghost_button: Element<'_, Message> = tooltip(
            button(text("G").size(10).font(iced::Font::MONOSPACE).center())
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
                    .font(iced::Font::MONOSPACE)
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
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .width(WALLET_COLUMN_WIDTH)
            .into()
    }

    pub(super) fn view_tracked_trade_coin_cell(&self, coin: String) -> Element<'_, Message> {
        let theme = self.theme();
        let mut coin_content = row![];
        if let Some(icon) = helpers::symbol_icon(&coin, 14, theme.palette().text) {
            coin_content = coin_content.push(icon).push(Space::new().width(4.0));
        }
        coin_content = coin_content
            .push(text(coin.clone()).size(12).font(iced::Font::MONOSPACE))
            .align_y(iced::Alignment::Center);

        button(coin_content)
            .on_press(Message::SymbolSelected(coin))
            .padding(0)
            .width(80)
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
            })
            .into()
    }
}
