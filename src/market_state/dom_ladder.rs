use crate::api::OrderBook;
use crate::helpers::{aggregate_levels, valid_book_tick_size};

// ---------------------------------------------------------------------------
// DOM Ladder Data
//
// Pure data computation for the DOM ladder view. Lives in `market_state`
// rather than `market_views` so the same `OrderBookInstance` that owns the
// raw book can also own a cache of these derived rows, keyed by the book
// revision + tick + row count. The view layer in
// `market_views::order_book::depth::dom` consumes borrowed cached rows
// and only renders.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct DomLadderRow {
    pub price: f64,
    pub bid_size: Option<f64>,
    pub bid_cumulative: Option<f64>,
    pub ask_size: Option<f64>,
    pub ask_cumulative: Option<f64>,
    pub is_best_bid: bool,
    pub is_best_ask: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DomLadderRows {
    pub asks: Vec<DomLadderRow>,
    pub bids: Vec<DomLadderRow>,
    pub max_size: f64,
    pub max_cumulative: f64,
}

pub fn build_dom_ladder_rows(book: &OrderBook, tick: f64, side_rows: usize) -> DomLadderRows {
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

/// Cache key for the DOM-ladder derivation. The view path looks up the
/// cached rows via this triple; a mismatch on any field invalidates and
/// recomputes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DomLadderCacheKey {
    pub(crate) book_revision: u64,
    pub(crate) tick_bits: u64,
    pub(crate) side_rows: usize,
}

#[derive(Debug, Default)]
pub(crate) struct DomLadderCache {
    pub(crate) key: DomLadderCacheKey,
    pub(crate) rows: DomLadderRows,
    /// `false` until the first compute, so the default zero key doesn't
    /// accidentally match a real `(book_revision=0, tick=0)` lookup.
    pub(crate) populated: bool,
}
