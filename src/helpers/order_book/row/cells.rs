use crate::helpers::{format_decimal_with_commas, format_size};
use crate::message::Message;

use super::marker::user_order_price_marker;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

/// Level sizes and totals share one formatter across the depth list and the
/// DOM ladder. Outcome books trade whole contracts, so they drop the
/// fractional digits; `whole_contracts: false` keeps the existing formatting.
pub fn format_book_size(size: f64, whole_contracts: bool) -> String {
    if whole_contracts {
        format!("{size:.0}")
    } else {
        format_size(size)
    }
}

pub fn format_book_total(total: f64, whole_contracts: bool, total_in_notional: bool) -> String {
    if total_in_notional {
        let amount = if total >= 10_000.0 {
            format_size(total)
        } else {
            format_decimal_with_commas(total, 2)
        };
        format!("${amount}")
    } else {
        format_book_size(total, whole_contracts)
    }
}

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
    .into()
}

pub(super) fn size_cell(sz: f64, alpha: f32, whole_contracts: bool) -> Element<'static, Message> {
    text(format_book_size(sz, whole_contracts))
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

pub(super) fn total_cell(
    cum: f64,
    whole_contracts: bool,
    total_in_notional: bool,
) -> Element<'static, Message> {
    container(
        text(format_book_total(cum, whole_contracts, total_in_notional))
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

#[cfg(test)]
mod tests;
