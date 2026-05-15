mod dom;
mod spread;

use super::UserOrderBookLevels;
use crate::app_state::TradingTerminal;
use crate::helpers::{BookRowData, book_row, tick_decimals};
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::{Column, column, container, responsive, scrollable};
use iced::{Element, Fill, Theme};

const CENTERED_ORDER_BOOK_ROW_HEIGHT: f32 = 20.0;

// ---------------------------------------------------------------------------
// Order Book Depth
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_order_book_rows(
        id: OrderBookId,
        inst: &OrderBookInstance,
        tick: f64,
        theme: &Theme,
        user_order_levels: &UserOrderBookLevels,
    ) -> Element<'static, Message> {
        let max_levels = 100;
        let decimals = tick_decimals(tick);

        let depth = inst.aggregated_depth(tick);

        // Asks come out of the cache in ascending price (inside-out); the
        // display wants worst-at-top, best-at-bottom above the spread line.
        let ask_rows: Vec<(f64, f64, f64)> = depth
            .asks
            .iter()
            .take(max_levels)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let max_ask_cum = ask_rows.last().map(|(_, _, cum)| *cum).unwrap_or(1.0);

        let bid_rows: Vec<(f64, f64, f64)> = depth.bids.iter().take(max_levels).copied().collect();
        let max_bid_cum = bid_rows.last().map(|(_, _, cum)| *cum).unwrap_or(1.0);

        let max_cum = max_ask_cum.max(max_bid_cum).max(1.0);
        let max_sz = ask_rows
            .iter()
            .chain(bid_rows.iter())
            .map(|(_, size, _)| *size)
            .fold(0.0f64, |max_seen, size| max_seen.max(size))
            .max(1.0);

        let spread_widget = Self::view_order_book_spread_widget(id, inst, theme);
        let row_padding = iced::Padding {
            top: 0.0,
            right: 15.0,
            bottom: 0.0,
            left: 0.0,
        };

        if inst.center_on_mid {
            let centered_asks = ask_rows.clone();
            let centered_bids = bid_rows.clone();
            let centered_ask_orders = user_order_levels.clone();
            let centered_bid_orders = user_order_levels.clone();

            let order_book_rows = column![
                responsive(move |size| {
                    let count =
                        centered_order_book_side_row_count(size.height, centered_asks.len());
                    let start = centered_asks.len().saturating_sub(count);
                    let asks = depth_ask_column(
                        id,
                        &centered_asks[start..],
                        tick,
                        max_cum,
                        max_sz,
                        decimals,
                        &centered_ask_orders,
                    );

                    container(asks)
                        .height(Fill)
                        .align_y(iced::alignment::Vertical::Bottom)
                        .into()
                })
                .height(Fill),
                spread_widget,
                responsive(move |size| {
                    let count =
                        centered_order_book_side_row_count(size.height, centered_bids.len());
                    let bids = depth_bid_column(
                        id,
                        &centered_bids[..count],
                        tick,
                        max_cum,
                        max_sz,
                        decimals,
                        &centered_bid_orders,
                    );

                    container(bids)
                        .height(Fill)
                        .align_y(iced::alignment::Vertical::Top)
                        .into()
                })
                .height(Fill),
            ]
            .height(Fill)
            .spacing(2);

            return container(order_book_rows)
                .width(Fill)
                .height(Fill)
                .padding(row_padding)
                .clip(true)
                .into();
        }

        let asks = depth_ask_column(
            id,
            &ask_rows,
            tick,
            max_cum,
            max_sz,
            decimals,
            user_order_levels,
        );
        let bids = depth_bid_column(
            id,
            &bid_rows,
            tick,
            max_cum,
            max_sz,
            decimals,
            user_order_levels,
        );
        let order_book_rows = column![asks, spread_widget, bids].spacing(2);

        scrollable(
            container(order_book_rows)
                .width(Fill)
                .padding(row_padding),
        )
        .height(Fill)
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(4.0)
                .scroller_width(4.0)
                .margin(2.0),
        ))
        .id(inst.scroll_id.clone())
        .into()
    }

    pub(super) fn view_order_book_dom_ladder(
        id: OrderBookId,
        inst: &OrderBookInstance,
        tick: f64,
        theme: &Theme,
        user_order_levels: &UserOrderBookLevels,
    ) -> Element<'static, Message> {
        let spread_widget = Self::view_order_book_spread_widget(id, inst, theme);
        dom::view_order_book_dom_ladder(id, inst, tick, spread_widget, user_order_levels)
    }

    pub(super) fn view_order_book_dom_header() -> Element<'static, Message> {
        dom::view_order_book_dom_header()
    }
}

pub(super) fn centered_order_book_side_row_count(
    side_height: f32,
    available_rows: usize,
) -> usize {
    if side_height <= 0.0 {
        return 0;
    }

    ((side_height / CENTERED_ORDER_BOOK_ROW_HEIGHT).floor() as usize).min(available_rows)
}

fn depth_ask_column(
    id: OrderBookId,
    rows: &[(f64, f64, f64)],
    tick: f64,
    max_cum: f64,
    max_sz: f64,
    decimals: usize,
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
                    has_user_order: user_order_levels.has_ask_at_price(px, tick),
                },
                max_cum,
                max_sz,
                decimals,
                false,
                Message::OrderBookPriceSelected {
                    id,
                    price: format!("{px:.decimals$}"),
                },
            ))
        })
}

fn depth_bid_column(
    id: OrderBookId,
    rows: &[(f64, f64, f64)],
    tick: f64,
    max_cum: f64,
    max_sz: f64,
    decimals: usize,
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
                    has_user_order: user_order_levels.has_bid_at_price(px, tick),
                },
                max_cum,
                max_sz,
                decimals,
                true,
                Message::OrderBookPriceSelected {
                    id,
                    price: format!("{px:.decimals$}"),
                },
            ))
        })
}
