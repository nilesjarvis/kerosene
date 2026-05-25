use super::super::UserOrderBookLevels;
use crate::helpers::{BookRowData, book_row};
use crate::market_state::OrderBookId;
use crate::message::Message;

use iced::widget::Column;

const CENTERED_ORDER_BOOK_ROW_HEIGHT: f32 = 20.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct DepthColumnContext {
    pub(super) id: OrderBookId,
    pub(super) tick: f64,
    pub(super) max_cum: f64,
    pub(super) max_sz: f64,
    pub(super) decimals: usize,
    pub(super) reverse_side: bool,
}

// ---------------------------------------------------------------------------
// Depth Rows
// ---------------------------------------------------------------------------

pub(super) fn max_cumulative_depth(rows: &[(f64, f64, f64)]) -> f64 {
    rows.iter()
        .map(|(_, _, cum)| *cum)
        .fold(0.0f64, f64::max)
        .max(1.0)
}

pub(super) fn max_level_size(asks: &[(f64, f64, f64)], bids: &[(f64, f64, f64)]) -> f64 {
    asks.iter()
        .chain(bids.iter())
        .map(|(_, size, _)| *size)
        .fold(0.0f64, f64::max)
        .max(1.0)
}

pub(super) fn centered_order_book_side_row_count(side_height: f32, available_rows: usize) -> usize {
    if side_height <= 0.0 {
        return 0;
    }

    ((side_height / CENTERED_ORDER_BOOK_ROW_HEIGHT).floor() as usize).min(available_rows)
}

pub(super) fn order_book_row_padding() -> iced::Padding {
    iced::Padding {
        top: 0.0,
        right: 15.0,
        bottom: 0.0,
        left: 0.0,
    }
}

pub(super) fn order_book_scroll_direction() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new()
            .width(4.0)
            .scroller_width(4.0)
            .margin(2.0),
    )
}

pub(super) fn depth_ask_column(
    context: DepthColumnContext,
    rows: &[(f64, f64, f64)],
    user_order_levels: &UserOrderBookLevels,
) -> Column<'static, Message> {
    rows.iter()
        .copied()
        .fold(Column::new().spacing(0), |col, (px, size, cum)| {
            col.push(book_row(
                BookRowData {
                    px,
                    sz: size,
                    cum,
                    has_user_order: user_order_levels.has_ask_at_price(px, context.tick),
                },
                context.max_cum,
                context.max_sz,
                context.decimals,
                false,
                context.reverse_side,
                Message::OrderBookPriceSelected {
                    id: context.id,
                    price: format!("{:.decimals$}", px, decimals = context.decimals),
                },
            ))
        })
}

pub(super) fn depth_bid_column(
    context: DepthColumnContext,
    rows: &[(f64, f64, f64)],
    user_order_levels: &UserOrderBookLevels,
) -> Column<'static, Message> {
    rows.iter()
        .copied()
        .fold(Column::new().spacing(0), |col, (px, size, cum)| {
            col.push(book_row(
                BookRowData {
                    px,
                    sz: size,
                    cum,
                    has_user_order: user_order_levels.has_bid_at_price(px, context.tick),
                },
                context.max_cum,
                context.max_sz,
                context.decimals,
                true,
                context.reverse_side,
                Message::OrderBookPriceSelected {
                    id: context.id,
                    price: format!("{:.decimals$}", px, decimals = context.decimals),
                },
            ))
        })
}
