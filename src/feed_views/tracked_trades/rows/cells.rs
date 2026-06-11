use crate::app_state::TradingTerminal;
use crate::feed_views::tracked_trades::layout::{COIN_WIDTH, WALLET_COLUMN_WIDTH};
use crate::helpers;
use crate::message::Message;
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, row, text, tooltip};
use iced::{Element, Theme};

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
        let tooltip_label = if display.has_label {
            format!("{} ({address})", display.primary)
        } else {
            format!("Copy {address}")
        };

        wallet_address_action_cell(WalletAddressActionCell {
            address: address.clone(),
            label: display.primary,
            tooltip_label,
            hover_key: format!("tracked-trades:{address}"),
            hovered_key: self.hovered_wallet_address_actions.as_deref(),
            width: WALLET_COLUMN_WIDTH,
            text_size: 12,
            text_color: theme.palette().primary,
        })
    }

    pub(super) fn view_tracked_trade_coin_cell(&self, coin: String) -> Element<'_, Message> {
        let theme = self.theme();
        let display_coin = self.display_coin_for_journal(&coin);
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
