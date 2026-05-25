use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::message::Message;

use super::formatting::{TickerTapeItem, pct_color, percent_label, price_label};
use super::{
    TICKER_TAPE_HEIGHT, TICKER_TAPE_ICON_SIZE, TICKER_TAPE_ITEM_HORIZONTAL_PADDING,
    TICKER_TAPE_ITEM_SPACING, TICKER_TAPE_SEPARATOR_WIDTH,
};
use iced::widget::container as container_style;
use iced::widget::{button, container, row, rule, text};
use iced::{Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Ticker Tape Components
// ---------------------------------------------------------------------------

pub(super) fn ticker_tape_item(
    item: &TickerTapeItem,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    item_width: f32,
) -> Element<'static, Message> {
    let icon = helpers::symbol_icon(
        &item.symbol,
        TICKER_TAPE_ICON_SIZE as u16,
        theme.palette().text,
    )
    .map(Element::from)
    .unwrap_or_else(|| fallback_ticker_logo(&item.ticker, theme));
    let pct_color = pct_color(item.pct_24h, theme);
    let symbol = item.symbol.clone();

    let content = row![
        icon,
        text(item.ticker.clone())
            .size(12)
            .font(crate::app_fonts::monospace_font()),
        text(price_label(item.price, denomination))
            .size(12)
            .font(crate::app_fonts::monospace_font()),
        text(percent_label(item.pct_24h))
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .color(pct_color),
    ]
    .spacing(TICKER_TAPE_ITEM_SPACING)
    .align_y(iced::Alignment::Center);

    let centered_content = container(content)
        .width(Fill)
        .height(Length::Fixed(TICKER_TAPE_HEIGHT))
        .center_y(Length::Fixed(TICKER_TAPE_HEIGHT));

    container(
        button(centered_content)
            .on_press(Message::SymbolSelected(symbol))
            .width(Fill)
            .height(Length::Fixed(TICKER_TAPE_HEIGHT))
            .padding([0, TICKER_TAPE_ITEM_HORIZONTAL_PADDING])
            .style(|theme: &Theme, status| ticker_tape_item_button_style(theme, status)),
    )
    .width(Length::Fixed(item_width))
    .center_y(Length::Fixed(TICKER_TAPE_HEIGHT))
    .clip(true)
    .into()
}

pub(super) fn ticker_tape_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.10,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .width(Length::Fixed(TICKER_TAPE_SEPARATOR_WIDTH))
    .height(18)
    .center_y(Length::Fixed(TICKER_TAPE_HEIGHT))
    .into()
}

pub(super) fn ticker_tape_bar_style(theme: &Theme, corner_radius: f32) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.strong.text;
    border_color.a = 0.10;

    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        text_color: Some(theme.palette().text),
        border: iced::Border {
            width: 1.0,
            color: border_color,
            radius: corner_radius.into(),
        },
        ..Default::default()
    }
}

fn fallback_ticker_logo(ticker: &str, theme: &Theme) -> Element<'static, Message> {
    let label = ticker
        .chars()
        .find(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());
    let color = theme.palette().primary;

    container(
        text(label)
            .size(9)
            .font(crate::app_fonts::monospace_font())
            .color(color)
            .center(),
    )
    .center_x(Length::Fixed(TICKER_TAPE_ICON_SIZE))
    .center_y(Length::Fixed(TICKER_TAPE_ICON_SIZE))
    .style(move |_theme: &Theme| {
        let mut background = color;
        background.a = 0.12;

        container_style::Style {
            background: Some(background.into()),
            border: iced::Border {
                radius: (TICKER_TAPE_ICON_SIZE * 0.5).into(),
                width: 1.0,
                color,
            },
            ..Default::default()
        }
    })
    .into()
}

fn ticker_tape_item_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(theme.extended_palette().background.weak.color.into())
        }
        _ => None,
    };

    button::Style {
        background,
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 3.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}
