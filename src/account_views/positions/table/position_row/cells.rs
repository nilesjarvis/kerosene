use crate::helpers;
use crate::message::Message;
use crate::pnl_card::{PnlCardTarget, pnl_card_icon_button};

use iced::widget::{Space, button, row, text};
use iced::{Color, Element, Theme};

// ---------------------------------------------------------------------------
// Position Row Cells
// ---------------------------------------------------------------------------

pub(super) fn position_symbol_button<'a>(coin: &'a str, theme: &Theme) -> Element<'a, Message> {
    let coin_key = coin.to_string();
    let mut coin_content = row![];
    if let Some(icon) = helpers::symbol_icon(coin, 14, theme.palette().text) {
        coin_content = coin_content.push(icon).push(Space::new().width(4.0));
    }
    coin_content = coin_content
        .push(text(coin).size(12))
        .align_y(iced::Alignment::Center);

    button(coin_content)
        .on_press(Message::SymbolSelected(coin_key))
        .padding(0)
        .style(|theme: &Theme, status| {
            let text_color = match status {
                button::Status::Hovered => theme.palette().success,
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

pub(super) fn position_upnl_cell<'a>(
    coin: &str,
    upnl: String,
    color: Color,
) -> Element<'a, Message> {
    row![
        text(upnl)
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .color(color),
        pnl_card_icon_button(
            Some(Message::OpenPnlCard(PnlCardTarget::Position(
                coin.to_string()
            ))),
            "Open PnL card",
        ),
    ]
    .spacing(3)
    .align_y(iced::Alignment::Center)
    .into()
}
