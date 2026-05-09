mod spread;

use crate::app_state::TradingTerminal;
use crate::helpers::{aggregate_levels, book_row, tick_decimals};
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
    ) -> Element<'static, Message> {
        let max_levels = 100;
        let decimals = tick_decimals(tick);

        let agg_asks = aggregate_levels(&inst.book.asks, tick, false);
        let agg_bids = aggregate_levels(&inst.book.bids, tick, true);

        let ask_display: Vec<&(f64, f64)> = agg_asks.iter().take(max_levels).collect();
        let mut ask_cum = 0.0;
        let ask_rows: Vec<(f64, f64, f64)> = ask_display
            .iter()
            .map(|(px, sz)| {
                ask_cum += sz;
                (*px, *sz, ask_cum)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let max_ask_cum = ask_rows.last().map(|(_, _, cum)| *cum).unwrap_or(1.0);

        let mut bid_cum = 0.0;
        let bid_rows: Vec<(f64, f64, f64)> = agg_bids
            .iter()
            .take(max_levels)
            .map(|(px, sz)| {
                bid_cum += sz;
                (*px, *sz, bid_cum)
            })
            .collect();
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
                col.push(book_row(px, size, cum, max_cum, max_sz, decimals, false))
            });
        let spread_widget = Self::view_order_book_spread_widget(id, inst, theme);
        let bids = bid_rows
            .into_iter()
            .fold(Column::new().spacing(0), |col, (px, size, cum)| {
                col.push(book_row(px, size, cum, max_cum, max_sz, decimals, true))
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
}
