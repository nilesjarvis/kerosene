use super::super::UserOrderBookLevels;
use crate::helpers::{clickable_book_row, format_size, tick_decimals, user_order_price_marker};
use crate::market_state::{DomLadderRow, OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, container, responsive, row, scrollable, text};
use iced::{Color, Element, Fill, Theme};

const DOM_SIDE_ROWS: usize = 80;

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

    if inst.center_on_mid {
        let centered_asks = rows.asks.clone();
        let centered_bids = rows.bids.clone();
        let centered_ask_orders = user_order_levels.clone();
        let centered_bid_orders = user_order_levels.clone();
        let max_size = rows.max_size;
        let max_cumulative = rows.max_cumulative;

        let ladder = iced::widget::column![
            responsive(move |size| {
                let count =
                    super::centered_order_book_side_row_count(size.height, centered_asks.len());
                let start = centered_asks.len().saturating_sub(count);
                let asks = dom_rows_column(
                    id,
                    &centered_asks[start..],
                    max_size,
                    max_cumulative,
                    decimals,
                    tick,
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
                    super::centered_order_book_side_row_count(size.height, centered_bids.len());
                let bids = dom_rows_column(
                    id,
                    &centered_bids[..count],
                    max_size,
                    max_cumulative,
                    decimals,
                    tick,
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

        return container(ladder)
            .width(Fill)
            .height(Fill)
            .padding(super::order_book_row_padding())
            .clip(true)
            .into();
    }

    let asks = dom_rows_column(
        id,
        &rows.asks,
        rows.max_size,
        rows.max_cumulative,
        decimals,
        tick,
        user_order_levels,
    );
    let bids = dom_rows_column(
        id,
        &rows.bids,
        rows.max_size,
        rows.max_cumulative,
        decimals,
        tick,
        user_order_levels,
    );
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

pub(super) fn view_order_book_dom_header() -> Element<'static, Message> {
    row![
        header_cell("Bid Total"),
        header_cell("Bid Size"),
        header_cell("Price"),
        header_cell("Ask Size"),
        header_cell("Ask Total"),
    ]
    .spacing(3)
    .into()
}

fn header_cell(label: &'static str) -> Element<'static, Message> {
    text(label)
        .size(11)
        .width(Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

fn dom_rows_column(
    id: OrderBookId,
    rows: &[DomLadderRow],
    max_size: f64,
    max_cumulative: f64,
    decimals: usize,
    tick: f64,
    user_order_levels: &UserOrderBookLevels,
) -> Column<'static, Message> {
    rows.iter()
        .fold(Column::new().spacing(0), |column, ladder_row| {
            column.push(dom_row(
                id,
                ladder_row,
                max_size,
                max_cumulative,
                decimals,
                tick,
                user_order_levels,
            ))
        })
}

fn dom_row(
    id: OrderBookId,
    row_data: &DomLadderRow,
    max_size: f64,
    max_cumulative: f64,
    decimals: usize,
    tick: f64,
    user_order_levels: &UserOrderBookLevels,
) -> Element<'static, Message> {
    let has_user_bid = user_order_levels.has_bid_at_price(row_data.price, tick);
    let has_user_ask = user_order_levels.has_ask_at_price(row_data.price, tick);
    let user_order_side = if has_user_bid {
        Some(true)
    } else if has_user_ask {
        Some(false)
    } else {
        None
    };

    let content: Element<'static, Message> = row![
        dom_value_cell(row_data.bid_cumulative, max_cumulative, true, true),
        dom_value_cell(row_data.bid_size, max_size, true, false),
        price_cell(row_data, decimals, user_order_side),
        dom_value_cell(row_data.ask_size, max_size, false, false),
        dom_value_cell(row_data.ask_cumulative, max_cumulative, false, true),
    ]
    .spacing(3)
    .into();

    clickable_book_row(
        content,
        Message::OrderBookPriceSelected {
            id,
            price: format!("{:.decimals$}", row_data.price),
        },
    )
}

fn dom_value_cell(
    value: Option<f64>,
    max_value: f64,
    is_bid: bool,
    is_cumulative: bool,
) -> Element<'static, Message> {
    let label = value.map(format_size).unwrap_or_else(|| String::from(""));
    let intensity = value
        .map(|value| (value / max_value.max(1.0)).clamp(0.0, 1.0) as f32)
        .unwrap_or(0.0);
    let alpha_scale = if is_cumulative { 0.16 } else { 0.34 };
    let text_alpha = if value.is_some() { 0.92 } else { 0.22 };

    let content = text(label)
        .size(12)
        .font(iced::Font::MONOSPACE)
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
        .padding([2, 4])
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
            text(format!("{price:.decimals$}"))
                .size(12)
                .font(iced::Font::MONOSPACE)
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
    .padding([2, 4])
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

#[cfg(test)]
mod tests {
    use crate::api::{BookLevel, OrderBook};
    use crate::market_state::build_dom_ladder_rows;

    fn level(px: f64, sz: f64) -> BookLevel {
        BookLevel { px, sz }
    }

    #[test]
    fn dom_ladder_builds_contiguous_ask_and_bid_rows() {
        let book = OrderBook {
            bids: vec![level(99.0, 2.0), level(97.0, 3.0)],
            asks: vec![level(101.0, 4.0), level(103.0, 5.0)],
        };

        let rows = build_dom_ladder_rows(&book, 1.0, 4);

        assert_eq!(rows.asks.len(), 4);
        assert_eq!(rows.bids.len(), 4);
        assert_eq!(
            rows.asks.iter().map(|row| row.price).collect::<Vec<_>>(),
            vec![104.0, 103.0, 102.0, 101.0]
        );
        assert_eq!(
            rows.bids.iter().map(|row| row.price).collect::<Vec<_>>(),
            vec![99.0, 98.0, 97.0, 96.0]
        );
    }

    #[test]
    fn dom_ladder_puts_sizes_and_cumulative_totals_on_the_correct_side() {
        let book = OrderBook {
            bids: vec![level(99.0, 2.0), level(98.0, 3.0)],
            asks: vec![level(101.0, 4.0), level(102.0, 5.0)],
        };

        let rows = build_dom_ladder_rows(&book, 1.0, 2);

        assert_eq!(rows.asks[0].price, 102.0);
        assert_eq!(rows.asks[0].ask_size, Some(5.0));
        assert_eq!(rows.asks[0].ask_cumulative, Some(9.0));
        assert_eq!(rows.asks[1].price, 101.0);
        assert_eq!(rows.asks[1].ask_size, Some(4.0));
        assert_eq!(rows.asks[1].ask_cumulative, Some(4.0));
        assert_eq!(rows.asks[1].bid_size, None);

        assert_eq!(rows.bids[0].price, 99.0);
        assert_eq!(rows.bids[0].bid_size, Some(2.0));
        assert_eq!(rows.bids[0].bid_cumulative, Some(2.0));
        assert_eq!(rows.bids[1].price, 98.0);
        assert_eq!(rows.bids[1].bid_size, Some(3.0));
        assert_eq!(rows.bids[1].bid_cumulative, Some(5.0));
        assert_eq!(rows.bids[0].ask_size, None);
    }

    #[test]
    fn dom_ladder_handles_empty_or_invalid_inputs_without_panics() {
        let empty = build_dom_ladder_rows(&OrderBook::empty(), 1.0, 10);
        assert!(empty.asks.is_empty());
        assert!(empty.bids.is_empty());

        let invalid_tick = build_dom_ladder_rows(&OrderBook::empty(), 0.0, 10);
        assert!(invalid_tick.asks.is_empty());
        assert!(invalid_tick.bids.is_empty());
    }
}
