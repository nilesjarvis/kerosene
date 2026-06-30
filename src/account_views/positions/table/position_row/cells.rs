use crate::helpers;
use crate::message::Message;
use crate::pnl_card::PnlCardTarget;

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Color, Element, Theme};

// ---------------------------------------------------------------------------
// Position Row Cells
// ---------------------------------------------------------------------------

pub(super) fn position_symbol_button(
    coin: &str,
    label: String,
    exchange_label: Option<String>,
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

    let tooltip_label = exchange_label
        .as_ref()
        .map(|exchange| format!("HIP-3 exchange: {exchange}"));
    if let Some(exchange) = exchange_label {
        coin_content = coin_content
            .push(Space::new().width(3.0))
            .push(position_exchange_chip(exchange, theme));
    }

    let button = button(coin_content)
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
        });

    if let Some(label) = tooltip_label {
        tooltip(button, text(label).size(10), tooltip::Position::Top).into()
    } else {
        button.into()
    }
}

fn position_exchange_chip(exchange: String, theme: &Theme) -> Element<'static, Message> {
    container(
        text(exchange)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(theme.extended_palette().background.weak.text)
            .wrapping(Wrapping::None),
    )
    .padding([1, 5])
    .style(|theme: &Theme| {
        let weak_text = theme.extended_palette().background.weak.text;
        iced::widget::container::Style {
            background: Some(
                Color {
                    a: 0.06,
                    ..weak_text
                }
                .into(),
            ),
            border: iced::Border {
                radius: 3.0.into(),
                width: 1.0,
                color: Color {
                    a: 0.28,
                    ..weak_text
                },
            },
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
