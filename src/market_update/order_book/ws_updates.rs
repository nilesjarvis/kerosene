use crate::api::OrderBook;
use crate::market_state::OrderBookSymbolMode;
use crate::signing::ChaseOrder;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// WebSocket Book Update Decisions
// ---------------------------------------------------------------------------

const CHASE_PRICE_EPSILON: f64 = 1e-12;

pub(super) fn order_book_tracks_coin(
    mode: &OrderBookSymbolMode,
    active_symbol: &str,
    coin: &str,
) -> bool {
    match mode {
        OrderBookSymbolMode::Active => active_symbol == coin,
        OrderBookSymbolMode::Fixed(symbol) => symbol == coin,
    }
}

pub(super) fn best_chase_price(book: &OrderBook, is_buy: bool) -> Option<f64> {
    let price = if is_buy {
        book.bids.first().map(|level| level.px)
    } else {
        book.asks.first().map(|level| level.px)
    };
    price.filter(|px| px.is_finite() && *px > 0.0)
}

pub(super) fn chase_should_reprice(
    chase: &ChaseOrder,
    active_symbol: &str,
    coin: &str,
    best_px: Option<f64>,
) -> bool {
    chase.coin == coin
        && active_symbol == coin
        && !chase.cancel_in_flight
        && !chase.stop_requested
        && best_px.is_some_and(|px| (px - chase.current_price).abs() > CHASE_PRICE_EPSILON)
}
