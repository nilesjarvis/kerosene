use crate::helpers::format_size;
use crate::message::Message;

use super::marker::user_order_price_marker;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

pub(super) fn price_cell(
    px: f64,
    decimals: usize,
    has_user_order: bool,
    is_bid: bool,
) -> Element<'static, Message> {
    container(
        row![
            Space::new().width(Fill),
            user_order_price_marker(has_user_order.then_some(is_bid)),
            text(format!("{px:.decimals$}"))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .style(move |t: &Theme| text::Style {
                    color: Some(if is_bid {
                        t.palette().success
                    } else {
                        t.palette().danger
                    })
                }),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .into()
}

pub(super) fn size_cell(sz: f64, alpha: f32) -> Element<'static, Message> {
    text(format_size(sz))
        .size(12)
        .font(crate::app_fonts::monospace_font())
        .align_x(iced::alignment::Horizontal::Right)
        .style(move |theme: &Theme| text::Style {
            color: Some(Color {
                a: alpha,
                ..theme.palette().text
            }),
        })
        .width(Fill)
        .into()
}

pub(super) fn total_cell(cum: f64) -> Element<'static, Message> {
    container(
        text(format_size(cum))
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .align_x(iced::alignment::Horizontal::Right)
            .style(move |theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.weak.text),
            })
            .width(Fill),
    )
    .width(Fill)
    .into()
}
