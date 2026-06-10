use self::rows::{DomRowContext, dom_rows_column};
use super::super::UserOrderBookLevels;
use crate::helpers::{nice_step_ceil, tick_decimals};
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::{container, responsive, row, scrollable, text};
use iced::{Element, Fill};

const DOM_SIDE_ROWS: usize = 80;

mod rows;

// ---------------------------------------------------------------------------
// DOM Ladder View
// ---------------------------------------------------------------------------

pub(super) fn view_order_book_dom_ladder(
    id: OrderBookId,
    inst: &OrderBookInstance,
    tick: f64,
    spread_widget: Element<'static, Message>,
    user_order_levels: &UserOrderBookLevels,
) -> Element<'static, Message> {
    let rows = inst.dom_ladder_rows(tick, DOM_SIDE_ROWS);
    let decimals = tick_decimals(tick);
    let row_context = DomRowContext {
        id,
        // Quantized so cell heat does not rescale on every book update.
        max_size: nice_step_ceil(rows.max_size),
        max_cumulative: nice_step_ceil(rows.max_cumulative),
        decimals,
        tick,
        reverse_side: inst.reverse_side,
    };

    if inst.center_on_mid {
        let centered_asks = rows.asks.clone();
        let centered_bids = rows.bids.clone();
        let centered_ask_orders = user_order_levels.clone();
        let centered_bid_orders = user_order_levels.clone();

        let ladder = iced::widget::column![
            responsive(move |size| {
                let count =
                    super::centered_order_book_side_row_count(size.height, centered_asks.len());
                let start = centered_asks.len().saturating_sub(count);
                let asks =
                    dom_rows_column(&centered_asks[start..], row_context, &centered_ask_orders);

                container(asks)
                    .height(Fill)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .into()
            })
            .height(Fill),
            spread_widget,
            responsive(move |size| {
                let count =
                    super::centered_order_book_side_row_count(size.height, centered_bids.len());
                let bids =
                    dom_rows_column(&centered_bids[..count], row_context, &centered_bid_orders);

                container(bids)
                    .height(Fill)
                    .align_y(iced::alignment::Vertical::Top)
                    .into()
            })
            .height(Fill),
        ]
        .height(Fill)
        .spacing(2);

        return container(ladder)
            .width(Fill)
            .height(Fill)
            .padding(super::order_book_row_padding())
            .clip(true)
            .into();
    }

    let asks = dom_rows_column(&rows.asks, row_context, user_order_levels);
    let bids = dom_rows_column(&rows.bids, row_context, user_order_levels);
    let ladder = iced::widget::column![asks, spread_widget, bids].spacing(2);

    scrollable(
        container(ladder)
            .width(Fill)
            .padding(super::order_book_row_padding()),
    )
    .height(Fill)
    .direction(super::order_book_scroll_direction())
    .id(inst.scroll_id.clone())
    .into()
}

pub(super) fn view_order_book_dom_header(reverse_side: bool) -> Element<'static, Message> {
    let labels = if reverse_side {
        ["Ask Total", "Ask Size", "Price", "Bid Size", "Bid Total"]
    } else {
        ["Bid Total", "Bid Size", "Price", "Ask Size", "Ask Total"]
    };

    // Mirror the data rows' insets (15px scrollbar gutter outside, 4px
    // horizontal padding per cell) so the labels line up with the numbers.
    container(
        row![
            header_cell(labels[0]),
            header_cell(labels[1]),
            header_cell(labels[2]),
            header_cell(labels[3]),
            header_cell(labels[4]),
        ]
        .spacing(3),
    )
    .padding(super::order_book_row_padding())
    .into()
}

fn header_cell(label: &'static str) -> Element<'static, Message> {
    container(
        text(label)
            .size(11)
            .width(Fill)
            .align_x(iced::alignment::Horizontal::Right),
    )
    .width(Fill)
    .padding([0, 4])
    .into()
}

#[cfg(test)]
mod tests;
