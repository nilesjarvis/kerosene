mod dom;
mod spread;

use super::UserOrderBookLevels;
use crate::app_state::TradingTerminal;
use crate::helpers::{BookRowData, book_row, tick_decimals};
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::{Column, column, container, scrollable};
use iced::{Element, Fill, Theme};

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

        let asks = ask_rows
            .into_iter()
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
            });
        let spread_widget = Self::view_order_book_spread_widget(id, inst, theme);
        let bids = bid_rows
            .into_iter()
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
            });

        let order_book_rows = column![asks, spread_widget, bids].spacing(2);

        scrollable(
            container(order_book_rows)
                .width(Fill)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 15.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
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
