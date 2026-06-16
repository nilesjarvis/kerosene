mod dom;
mod rows;
mod spread;

use super::UserOrderBookLevels;
use crate::app_state::TradingTerminal;
use crate::helpers::{nice_step_ceil, tick_decimals};
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::{column, container, responsive, scrollable};
use iced::{Element, Fill, Theme};

use rows::{
    DepthColumnContext, centered_order_book_side_row_count, depth_ask_column, depth_bid_column,
    max_cumulative_depth, max_level_size, order_book_row_padding, order_book_scroll_direction,
};

/// Fixed number of rows rendered per side. The sides are padded with inert
/// filler rows up to this count so the scroll content height never changes
/// while the market moves.
const DEPTH_SIDE_ROWS: usize = 40;

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
        whole_contracts: bool,
    ) -> Element<'static, Message> {
        let decimals = tick_decimals(tick);

        let depth = inst.aggregated_depth(tick);

        // Asks come out of the cache in ascending price (inside-out); the
        // display wants worst-at-top, best-at-bottom above the spread line.
        let ask_rows: Vec<(f64, f64, f64)> = depth
            .asks
            .iter()
            .take(DEPTH_SIDE_ROWS)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let max_ask_cum = max_cumulative_depth(&ask_rows);

        let bid_rows: Vec<(f64, f64, f64)> =
            depth.bids.iter().take(DEPTH_SIDE_ROWS).copied().collect();
        let max_bid_cum = max_cumulative_depth(&bid_rows);

        // Quantize the normalizers to a 1-2-5 step so bars and heat do not
        // rescale on every book update.
        let max_cum = nice_step_ceil(max_ask_cum.max(max_bid_cum));
        let max_sz = nice_step_ceil(max_level_size(&ask_rows, &bid_rows));

        let spread_widget = Self::view_order_book_spread_widget(id, inst, theme);
        let column_context = DepthColumnContext {
            id,
            tick,
            max_cum,
            max_sz,
            decimals,
            whole_contracts,
            reverse_side: inst.reverse_side,
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
                        column_context,
                        &centered_asks[start..],
                        &centered_ask_orders,
                        0,
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
                        column_context,
                        &centered_bids[..count],
                        &centered_bid_orders,
                        0,
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
                .padding(order_book_row_padding())
                .clip(true)
                .into();
        }

        let asks = depth_ask_column(
            column_context,
            &ask_rows,
            user_order_levels,
            DEPTH_SIDE_ROWS,
        );
        let bids = depth_bid_column(
            column_context,
            &bid_rows,
            user_order_levels,
            DEPTH_SIDE_ROWS,
        );
        let order_book_rows = column![asks, spread_widget, bids].spacing(2);

        scrollable(
            container(order_book_rows)
                .width(Fill)
                .padding(order_book_row_padding()),
        )
        .height(Fill)
        .direction(order_book_scroll_direction())
        .id(inst.scroll_id.clone())
        .into()
    }

    pub(super) fn view_order_book_dom_ladder(
        id: OrderBookId,
        inst: &OrderBookInstance,
        tick: f64,
        theme: &Theme,
        user_order_levels: &UserOrderBookLevels,
        whole_contracts: bool,
    ) -> Element<'static, Message> {
        let spread_widget = Self::view_order_book_spread_widget(id, inst, theme);
        dom::view_order_book_dom_ladder(
            id,
            inst,
            tick,
            spread_widget,
            user_order_levels,
            whole_contracts,
        )
    }

    pub(super) fn view_order_book_dom_header(reverse_side: bool) -> Element<'static, Message> {
        dom::view_order_book_dom_header(reverse_side)
    }

    pub(super) fn view_order_book_depth_chart(
        id: OrderBookId,
        inst: &OrderBookInstance,
        tick: f64,
        user_order_levels: &UserOrderBookLevels,
        whole_contracts: bool,
    ) -> Element<'static, Message> {
        let depth = inst.aggregated_depth(tick);
        // The aggregation cache hands out a RefCell guard, so the canvas gets
        // owned copies of the (small) level vectors.
        let bids = depth.bids.clone();
        let asks = depth.asks.clone();

        // Mid from the visible top of book so the marker agrees with what the
        // other display modes show for the same data.
        let (best_bid, best_ask) = inst.visible_best_bid_ask();
        let mid = match (best_bid, best_ask) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            (bid, ask) => bid.or(ask),
        };

        // The user-order levels store tick-bucket keys; rebuild the bucket
        // prices the chart's grid uses.
        let user_bid_prices: Vec<f64> = user_order_levels
            .bids
            .iter()
            .map(|&key| key as f64 * tick)
            .collect();
        let user_ask_prices: Vec<f64> = user_order_levels
            .asks
            .iter()
            .map(|&key| key as f64 * tick)
            .collect();

        iced::widget::canvas(crate::depth_chart::DepthChart {
            id,
            bids,
            asks,
            mid,
            tick,
            decimals: tick_decimals(tick),
            whole_contracts,
            user_bid_prices,
            user_ask_prices,
        })
        .width(Fill)
        .height(Fill)
        .into()
    }
}

#[cfg(test)]
mod tests;
