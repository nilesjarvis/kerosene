use crate::app_state::TradingTerminal;
use crate::feed_views::liquidations::layout::{COIN_WIDTH, USER_WIDTH};
use crate::helpers;
use crate::message::Message;

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, row, text, tooltip};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_liquidated_user_cell(
        &self,
        liquidated_user: String,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let user = liquidated_user.trim();
        if user.is_empty() {
            return text("-")
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text)
                .wrapping(Wrapping::None)
                .width(USER_WIDTH)
                .into();
        }

        let user_tooltip = user.to_string();
        let display = self.wallet_display(user);
        let user_button: Element<'_, Message> = button(
            text(display.primary)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().primary)
                .wrapping(Wrapping::None),
        )
        .on_press(Message::CopyToClipboard(liquidated_user))
        .padding(0)
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .width(USER_WIDTH)
        .into();

        tooltip(
            user_button,
            text(user_tooltip).size(10).font(crate::app_fonts::monospace_font()),
            iced::widget::tooltip::Position::Top,
        )
        .into()
    }
}

pub(super) fn liquidation_symbol_button(
    coin: String,
    theme: &Theme,
) -> button::Button<'static, Message> {
    let mut coin_content = row![];
    if let Some(icon) = helpers::symbol_icon(&coin, 14, theme.palette().text) {
        coin_content = coin_content.push(icon).push(Space::new().width(4.0));
    }
    coin_content = coin_content
        .push(
            text(coin.clone())
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .wrapping(Wrapping::None),
        )
        .align_y(iced::Alignment::Center);

    button(coin_content)
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
        })
}
