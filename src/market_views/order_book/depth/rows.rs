use super::super::UserOrderBookLevels;
use crate::helpers::{BOOK_ROW_HEIGHT, BookRowData, book_row, placeholder_book_row};
use crate::market_state::OrderBookId;
use crate::message::Message;

use iced::widget::Column;

#[derive(Debug, Clone, Copy)]
pub(super) struct DepthColumnContext {
    pub(super) id: OrderBookId,
    pub(super) tick: f64,
    pub(super) max_cum: f64,
    pub(super) max_sz: f64,
    pub(super) decimals: usize,
    pub(super) whole_contracts: bool,
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

    ((side_height / BOOK_ROW_HEIGHT).floor() as usize).min(available_rows)
}

/// Rows to render per side in a centered (pinned-spread) layout. Both sides
/// must show an identical count so the book stays symmetric about the spread
/// and never lists more depth on one side than the other when aggregation or
/// scope drift leaves the sides with different numbers of levels. The
/// height-derived capacity is therefore clamped to the *thinner* side's
/// available rows, keeping the count equal as the market moves in either
/// direction instead of one side shrinking independently.
pub(super) fn centered_symmetric_side_row_count(
    side_height: f32,
    asks_available: usize,
    bids_available: usize,
) -> usize {
    centered_order_book_side_row_count(side_height, asks_available.min(bids_available))
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

/// Number of inert filler rows needed to bring a side up to `pad_to` rows.
/// Keeping both sides at a constant row count keeps the scroll content height
/// stable while levels appear and disappear, so nothing shifts under the
/// cursor and the load-time snap to 50% lands exactly on the spread row.
pub(super) fn side_padding_row_count(rendered_rows: usize, pad_to: usize) -> usize {
    pad_to.saturating_sub(rendered_rows)
}

pub(super) fn depth_ask_column(
    context: DepthColumnContext,
    rows: &[(f64, f64, f64)],
    user_order_levels: &UserOrderBookLevels,
    pad_to: usize,
) -> Column<'static, Message> {
    // Asks render worst-at-top, so filler rows go above the data.
    let mut col = Column::new().spacing(0);
    for _ in 0..side_padding_row_count(rows.len(), pad_to) {
        col = col.push(placeholder_book_row());
    }
    rows.iter().copied().fold(col, |col, (px, size, cum)| {
        col.push(book_row(
            BookRowData {
                px,
                sz: size,
                cum,
                has_user_order: user_order_levels.has_ask_at_price(px, context.tick),
                whole_contracts: context.whole_contracts,
            },
            context.max_cum,
            context.max_sz,
            context.decimals,
            false,
            context.reverse_side,
            Message::OrderBookPriceSelected {
                id: context.id,
                price: format!("{:.decimals$}", px, decimals = context.decimals).into(),
            },
        ))
    })
}

pub(super) fn depth_bid_column(
    context: DepthColumnContext,
    rows: &[(f64, f64, f64)],
    user_order_levels: &UserOrderBookLevels,
    pad_to: usize,
) -> Column<'static, Message> {
    let col = rows
        .iter()
        .copied()
        .fold(Column::new().spacing(0), |col, (px, size, cum)| {
            col.push(book_row(
                BookRowData {
                    px,
                    sz: size,
                    cum,
                    has_user_order: user_order_levels.has_bid_at_price(px, context.tick),
                    whole_contracts: context.whole_contracts,
                },
                context.max_cum,
                context.max_sz,
                context.decimals,
                true,
                context.reverse_side,
                Message::OrderBookPriceSelected {
                    id: context.id,
                    price: format!("{:.decimals$}", px, decimals = context.decimals).into(),
                },
            ))
        });
    // Bids render best-at-top, so filler rows go below the data.
    let mut col = col;
    for _ in 0..side_padding_row_count(rows.len(), pad_to) {
        col = col.push(placeholder_book_row());
    }
    col
}
