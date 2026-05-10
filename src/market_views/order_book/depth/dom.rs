use crate::api::OrderBook;
use crate::helpers::{aggregate_levels, format_size, tick_decimals, valid_book_tick_size};
use crate::market_state::OrderBookInstance;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, row, scrollable, text};
use iced::{Color, Element, Fill, Theme};

const DOM_SIDE_ROWS: usize = 80;

// ---------------------------------------------------------------------------
// DOM Ladder Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct DomLadderRow {
    price: f64,
    bid_size: Option<f64>,
    bid_cumulative: Option<f64>,
    ask_size: Option<f64>,
    ask_cumulative: Option<f64>,
    is_best_bid: bool,
    is_best_ask: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct DomLadderRows {
    asks: Vec<DomLadderRow>,
    bids: Vec<DomLadderRow>,
    max_size: f64,
    max_cumulative: f64,
}

fn build_dom_ladder_rows(book: &OrderBook, tick: f64, side_rows: usize) -> DomLadderRows {
    if !valid_book_tick_size(tick) || side_rows == 0 {
        return DomLadderRows {
            asks: Vec::new(),
            bids: Vec::new(),
            max_size: 1.0,
            max_cumulative: 1.0,
        };
    }

    let ask_levels = aggregate_levels(&book.asks, tick, false);
    let bid_levels = aggregate_levels(&book.bids, tick, true);
    let ask_map = level_map(&ask_levels, tick);
    let bid_map = level_map(&bid_levels, tick);

    let asks = ask_levels
        .first()
        .map(|(best_ask, _)| ask_rows(&ask_map, price_key(*best_ask, tick), tick, side_rows))
        .unwrap_or_default();
    let bids = bid_levels
        .first()
        .map(|(best_bid, _)| bid_rows(&bid_map, price_key(*best_bid, tick), tick, side_rows))
        .unwrap_or_default();

    let max_size = asks
        .iter()
        .chain(bids.iter())
        .filter_map(|row| row.bid_size.or(row.ask_size))
        .fold(0.0f64, f64::max)
        .max(1.0);
    let max_cumulative = asks
        .iter()
        .chain(bids.iter())
        .filter_map(|row| row.bid_cumulative.or(row.ask_cumulative))
        .fold(0.0f64, f64::max)
        .max(1.0);

    DomLadderRows {
        asks,
        bids,
        max_size,
        max_cumulative,
    }
}

fn level_map(levels: &[(f64, f64)], tick: f64) -> std::collections::BTreeMap<i64, f64> {
    levels
        .iter()
        .map(|(price, size)| (price_key(*price, tick), *size))
        .collect()
}

fn price_key(price: f64, tick: f64) -> i64 {
    (price / tick).round() as i64
}

fn ask_rows(
    ask_map: &std::collections::BTreeMap<i64, f64>,
    best_ask_key: i64,
    tick: f64,
    side_rows: usize,
) -> Vec<DomLadderRow> {
    let mut rows = Vec::with_capacity(side_rows);
    let mut cumulative = 0.0;
    for offset in 0..side_rows {
        let key = best_ask_key + offset as i64;
        let size = ask_map.get(&key).copied();
        if let Some(size) = size {
            cumulative += size;
        }
        rows.push(DomLadderRow {
            price: key as f64 * tick,
            bid_size: None,
            bid_cumulative: None,
            ask_size: size,
            ask_cumulative: (cumulative > 0.0).then_some(cumulative),
            is_best_bid: false,
            is_best_ask: offset == 0,
        });
    }
    rows.reverse();
    rows
}

fn bid_rows(
    bid_map: &std::collections::BTreeMap<i64, f64>,
    best_bid_key: i64,
    tick: f64,
    side_rows: usize,
) -> Vec<DomLadderRow> {
    let mut rows = Vec::with_capacity(side_rows);
    let mut cumulative = 0.0;
    for offset in 0..side_rows {
        let key = best_bid_key - offset as i64;
        let size = bid_map.get(&key).copied();
        if let Some(size) = size {
            cumulative += size;
        }
        rows.push(DomLadderRow {
            price: key as f64 * tick,
            bid_size: size,
            bid_cumulative: (cumulative > 0.0).then_some(cumulative),
            ask_size: None,
            ask_cumulative: None,
            is_best_bid: offset == 0,
            is_best_ask: false,
        });
    }
    rows
}

// ---------------------------------------------------------------------------
// DOM Ladder View
// ---------------------------------------------------------------------------

pub(super) fn view_order_book_dom_ladder(
    inst: &OrderBookInstance,
    tick: f64,
    spread_widget: Element<'static, Message>,
) -> Element<'static, Message> {
    let rows = build_dom_ladder_rows(&inst.book, tick, DOM_SIDE_ROWS);
    let decimals = tick_decimals(tick);
    let asks = rows
        .asks
        .iter()
        .fold(Column::new().spacing(0), |column, ladder_row| {
            column.push(dom_row(
                ladder_row,
                rows.max_size,
                rows.max_cumulative,
                decimals,
            ))
        });
    let bids = rows
        .bids
        .iter()
        .fold(Column::new().spacing(0), |column, ladder_row| {
            column.push(dom_row(
                ladder_row,
                rows.max_size,
                rows.max_cumulative,
                decimals,
            ))
        });

    let ladder = iced::widget::column![asks, spread_widget, bids].spacing(2);

    scrollable(container(ladder).width(Fill).padding(iced::Padding {
        top: 0.0,
        right: 15.0,
        bottom: 0.0,
        left: 0.0,
    }))
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

fn dom_row(
    row_data: &DomLadderRow,
    max_size: f64,
    max_cumulative: f64,
    decimals: usize,
) -> Element<'static, Message> {
    row![
        dom_value_cell(row_data.bid_cumulative, max_cumulative, true, true),
        dom_value_cell(row_data.bid_size, max_size, true, false),
        price_cell(row_data, decimals),
        dom_value_cell(row_data.ask_size, max_size, false, false),
        dom_value_cell(row_data.ask_cumulative, max_cumulative, false, true),
    ]
    .spacing(3)
    .into()
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

    container(
        text(label)
            .size(12)
            .font(iced::Font::MONOSPACE)
            .align_x(iced::alignment::Horizontal::Right)
            .style(move |theme: &Theme| text::Style {
                color: Some(Color {
                    a: text_alpha,
                    ..theme.palette().text
                }),
            })
            .width(Fill),
    )
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

fn price_cell(row_data: &DomLadderRow, decimals: usize) -> Element<'static, Message> {
    let price = row_data.price;
    let is_best_bid = row_data.is_best_bid;
    let is_best_ask = row_data.is_best_ask;
    container(
        text(format!("{price:.decimals$}"))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .align_x(iced::alignment::Horizontal::Right)
            .style(move |theme: &Theme| {
                let color = if is_best_bid {
                    theme.palette().success
                } else if is_best_ask {
                    theme.palette().danger
                } else {
                    theme.palette().text
                };
                text::Style { color: Some(color) }
            })
            .width(Fill),
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
    use super::*;
    use crate::api::BookLevel;

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
