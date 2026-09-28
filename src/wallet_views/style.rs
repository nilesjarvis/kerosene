use crate::message::Message;

use iced::widget::{Column, button, container, text};
use iced::{Color, Element, Fill, Length, Theme};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Wallet Detail Styling
// ---------------------------------------------------------------------------

pub(in crate::wallet_views) fn wallet_signed_value_color(
    value: Option<f64>,
    theme: &Theme,
) -> Color {
    match value {
        Some(value) if value > 0.0 => theme.palette().success,
        Some(value) if value < 0.0 => theme.palette().danger,
        Some(_) => theme.extended_palette().background.weak.text,
        None => theme.palette().warning,
    }
}

pub(in crate::wallet_views) fn wallet_symbol_button(
    label: String,
    symbol: String,
    width: f32,
    theme: &Theme,
) -> Element<'static, Message> {
    button(
        text(label)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(theme.palette().primary),
    )
    .on_press(Message::SymbolSelected(symbol))
    .padding(0)
    .width(Length::Fixed(width))
    .style(|_theme: &Theme, _status| button::Style {
        background: None,
        ..Default::default()
    })
    .into()
}

pub(super) fn wallet_detail_table(content: Column<'_, Message>) -> Element<'_, Message> {
    container(content)
        .padding([8, 8])
        .width(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(
                Color {
                    a: 0.22,
                    ..theme.extended_palette().background.weak.color
                }
                .into(),
            ),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
            ..Default::default()
        })
        .into()
}
