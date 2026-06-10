use crate::helpers::{format_decimal_with_commas, format_size};
use crate::message::Message;

use super::marker::user_order_price_marker;
use iced::widget::container as container_style;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

pub(super) fn price_cell(
    px: f64,
    decimals: usize,
    has_user_order: bool,
    is_bid: bool,
    is_best: bool,
) -> Element<'static, Message> {
    container(
        row![
            Space::new().width(Fill),
            user_order_price_marker(has_user_order.then_some(is_bid)),
            text(format_decimal_with_commas(px, decimals))
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
    .height(Fill)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |theme: &Theme| {
        // Mirror the DOM ladder's inside-market emphasis on the touch rows.
        let background = is_best.then(|| theme.extended_palette().background.weak.color.into());
        container_style::Style {
            background,
            ..Default::default()
        }
    })
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
                // The total column sits under the strongest part of the depth
                // gradient; a fixed-alpha variant of the main text color stays
                // readable there while keeping totals dimmer than sizes.
                color: Some(Color {
                    a: 0.62,
                    ..theme.palette().text
                }),
            })
            .width(Fill),
    )
    .width(Fill)
    .into()
}
