use crate::helpers;
use crate::message::Message;
use crate::pnl_card::PnlCardTarget;

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, row, text};
use iced::{Color, Element, Theme};

// ---------------------------------------------------------------------------
// Position Row Cells
// ---------------------------------------------------------------------------

pub(super) fn position_symbol_button(
    coin: &str,
    label: String,
    theme: &Theme,
) -> Element<'static, Message> {
    let coin_key = coin.to_string();
    let mut coin_content = row![];
    if let Some(icon) = helpers::symbol_icon(coin, 14, theme.palette().text) {
        coin_content = coin_content.push(icon).push(Space::new().width(4.0));
    }
    coin_content = coin_content
        .push(text(label).size(12).wrapping(Wrapping::None))
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

pub(super) fn position_upnl_cell(
    coin: &str,
    upnl: String,
    color: Color,
) -> Element<'static, Message> {
    let coin_key = coin.to_string();
    button(
        text(upnl)
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .color(color)
            .wrapping(Wrapping::None),
    )
    .on_press(Message::OpenPnlCard(PnlCardTarget::Position(coin_key)))
    .padding([1, 2])
    .style(move |theme: &Theme, status| {
        let mut text_color = color;
        let mut bg: Option<Color> = None;
        if status == button::Status::Hovered {
            text_color = theme.palette().text;
            bg = Some(Color { a: 0.12, ..color });
        }
        button::Style {
            background: bg.map(Into::into),
            text_color,
            ..Default::default()
        }
    })
    .into()
}
