use super::super::super::UserOrderBookLevels;
use crate::helpers::{
    BOOK_ROW_HEIGHT, clickable_book_row, format_book_size, format_decimal_with_commas,
    user_order_price_marker,
};
use crate::market_state::{DomLadderRow, OrderBookId};
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Column, Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// DOM Ladder Rows
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(super) struct DomRowContext {
    pub(super) id: OrderBookId,
    pub(super) max_size: f64,
    pub(super) max_cumulative: f64,
    pub(super) decimals: usize,
    pub(super) tick: f64,
    pub(super) whole_contracts: bool,
    pub(super) reverse_side: bool,
}

pub(super) fn dom_rows_column(
    rows: &[DomLadderRow],
    context: DomRowContext,
    user_order_levels: &UserOrderBookLevels,
) -> Column<'static, Message> {
    rows.iter()
        .fold(Column::new().spacing(0), |column, ladder_row| {
            column.push(dom_row(ladder_row, context, user_order_levels))
        })
}

fn dom_row(
    row_data: &DomLadderRow,
    context: DomRowContext,
    user_order_levels: &UserOrderBookLevels,
) -> Element<'static, Message> {
    let has_user_bid = user_order_levels.has_bid_at_price(row_data.price, context.tick);
    let has_user_ask = user_order_levels.has_ask_at_price(row_data.price, context.tick);
    let user_order_side = if has_user_bid {
        Some(true)
    } else if has_user_ask {
        Some(false)
    } else {
        None
    };

    let whole_contracts = context.whole_contracts;
    let bid_total = dom_value_cell(
        row_data.bid_cumulative,
        context.max_cumulative,
        true,
        true,
        whole_contracts,
    );
    let bid_size = dom_value_cell(
        row_data.bid_size,
        context.max_size,
        true,
        false,
        whole_contracts,
    );
    let price = price_cell(row_data, context.decimals, user_order_side);
    let ask_size = dom_value_cell(
        row_data.ask_size,
        context.max_size,
        false,
        false,
        whole_contracts,
    );
    let ask_total = dom_value_cell(
        row_data.ask_cumulative,
        context.max_cumulative,
        false,
        true,
        whole_contracts,
    );

    let content: Element<'static, Message> = if context.reverse_side {
        row![ask_total, ask_size, price, bid_size, bid_total]
    } else {
        row![bid_total, bid_size, price, ask_size, ask_total]
    }
    .spacing(3)
    .height(BOOK_ROW_HEIGHT)
    .into();

    clickable_book_row(
        content,
        Message::OrderBookPriceSelected {
            id: context.id,
            price: format!("{:.decimals$}", row_data.price, decimals = context.decimals).into(),
        },
    )
}

fn dom_value_cell(
    value: Option<f64>,
    max_value: f64,
    is_bid: bool,
    is_cumulative: bool,
    whole_contracts: bool,
) -> Element<'static, Message> {
    let label = value
        .map(|value| format_book_size(value, whole_contracts))
        .unwrap_or_default();
    // Same square-root heat curve as the depth list, so medium orders stay
    // visible instead of being flattened by one wall in the window.
    let intensity = value
        .map(|value| (value / max_value.max(1.0)).clamp(0.0, 1.0).powf(0.5) as f32)
        .unwrap_or(0.0);
    let alpha_scale = if is_cumulative { 0.16 } else { 0.34 };
    let text_alpha = if value.is_some() { 0.92 } else { 0.22 };

    let content = text(label)
        .size(12)
        .font(crate::app_fonts::monospace_font())
        .align_x(iced::alignment::Horizontal::Right)
        .style(move |theme: &Theme| text::Style {
            color: Some(Color {
                a: text_alpha,
                ..theme.palette().text
            }),
        })
        .width(Fill);

    container(content)
        .width(Fill)
        .height(Fill)
        .align_y(iced::alignment::Vertical::Center)
        .padding([0, 4])
        .style(move |theme: &Theme| {
            let mut color = if is_bid {
                theme.palette().success
            } else {
                theme.palette().danger
            };
            color.a = 0.03 + intensity * alpha_scale;
            container_style::Style {
                background: Some(color.into()),
                ..Default::default()
            }
        })
        .into()
}

fn price_cell(
    row_data: &DomLadderRow,
    decimals: usize,
    user_order_side: Option<bool>,
) -> Element<'static, Message> {
    let price = row_data.price;
    let is_best_bid = row_data.is_best_bid;
    let is_best_ask = row_data.is_best_ask;
    container(
        row![
            Space::new().width(Fill),
            user_order_price_marker(user_order_side),
            text(format_decimal_with_commas(price, decimals))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .style(move |theme: &Theme| {
                    let color = if is_best_bid {
                        theme.palette().success
                    } else if is_best_ask {
                        theme.palette().danger
                    } else {
                        theme.palette().text
                    };
                    text::Style { color: Some(color) }
                }),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .height(Fill)
    .align_y(iced::alignment::Vertical::Center)
    .padding([0, 4])
    .style(move |theme: &Theme| {
        let background = if is_best_bid || is_best_ask {
            Some(theme.extended_palette().background.weak.color.into())
        } else {
            None
        };
        container_style::Style {
            background,
            ..Default::default()
        }
    })
    .into()
}
