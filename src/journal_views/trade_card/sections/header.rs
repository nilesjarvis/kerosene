use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

#[allow(clippy::too_many_arguments)]
pub(in crate::journal_views::trade_card) fn journal_trade_card_header(
    coin_key: &str,
    display_coin: String,
    status: String,
    pnl: f64,
    status_color: Color,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let mut coin_row = row![];
    if let Some(icon) = helpers::symbol_icon(coin_key, 16, theme.palette().primary) {
        coin_row = coin_row.push(icon).push(Space::new().width(6.0));
    }

    coin_row = coin_row.push(text(display_coin).size(16).color(theme.palette().primary));

    let pnl_color = if pnl > 0.0 {
        theme.palette().success
    } else if pnl < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    };

    let pnl_str = denomination.format_signed_value(pnl, 2);

    row![
        coin_row.align_y(iced::Alignment::Center),
        Space::new().width(8.0),
        container(
            text(status)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(status_color)
        )
        .padding([2, 6])
        .style(move |_theme: &Theme| container_style::Style {
            border: iced::Border {
                color: Color {
                    a: 0.5,
                    ..status_color
                },
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        }),
        Space::new().width(Fill),
        text(pnl_str)
            .font(crate::app_fonts::monospace_font())
            .size(16)
            .color(pnl_color),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
