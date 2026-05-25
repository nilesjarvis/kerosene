use crate::api::{BookLevel, OrderBook};

// ---------------------------------------------------------------------------
// Order Book Scope Preservation
// ---------------------------------------------------------------------------

pub(super) fn merge_books_preserving_scope(current: &OrderBook, incoming: &OrderBook) -> OrderBook {
    OrderBook {
        bids: merge_book_side_preserving_scope(&current.bids, &incoming.bids, true),
        asks: merge_book_side_preserving_scope(&current.asks, &incoming.asks, false),
    }
}

fn merge_book_side_preserving_scope(
    current: &[BookLevel],
    incoming: &[BookLevel],
    is_bid: bool,
) -> Vec<BookLevel> {
    if incoming.is_empty() {
        return Vec::new();
    }
    if current.is_empty() {
        return incoming.to_vec();
    }

    let min_incoming = incoming
        .iter()
        .map(|level| level.px)
        .fold(f64::INFINITY, f64::min);
    let max_incoming = incoming
        .iter()
        .map(|level| level.px)
        .fold(f64::NEG_INFINITY, f64::max);

    let mut merged = Vec::with_capacity(current.len() + incoming.len());
    merged.extend(incoming.iter().cloned());
    merged.extend(
        current
            .iter()
            .filter(|level| {
                if is_bid {
                    level.px < min_incoming
                } else {
                    level.px > max_incoming
                }
            })
            .cloned(),
    );

    if is_bid {
        merged.sort_by(|a, b| b.px.partial_cmp(&a.px).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        merged.sort_by(|a, b| a.px.partial_cmp(&b.px).unwrap_or(std::cmp::Ordering::Equal));
    }
    merged
}
