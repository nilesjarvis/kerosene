use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::{button, text};
use iced::{Color, Element, Fill, Theme};

pub(super) fn view_order_book_spread_toggle<'a>(
    id: OrderBookId,
    inst: &'a OrderBookInstance,
) -> Element<'a, Message> {
    button(
        text(if inst.show_spread_chart {
            "Hide Spread Chart"
        } else {
            "Show Spread Chart"
        })
        .size(12)
        .center()
        .width(Fill),
    )
    .on_press(Message::ToggleOrderBookSpreadChart(id))
    .style(move |theme: &Theme, status| {
        let bg = if inst.show_spread_chart {
            theme.extended_palette().background.strong.color
        } else {
            match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.base.color,
            }
        };
        button::Style {
            background: Some(bg.into()),
            text_color: theme.palette().text,
            border: iced::Border {
                radius: 2.0.into(),
                width: if inst.show_spread_chart { 1.0 } else { 0.0 },
                color: if inst.show_spread_chart {
                    Color {
                        a: 0.5,
                        ..theme.palette().primary
                    }
                } else {
                    Color::TRANSPARENT
                },
            },
            ..Default::default()
        }
    })
    .into()
}
