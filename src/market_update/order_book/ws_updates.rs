use crate::api::OrderBook;
use crate::market_state::OrderBookSymbolMode;
use crate::signing::ChaseOrder;
use std::time::Instant;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// WebSocket Book Update Decisions
// ---------------------------------------------------------------------------

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
    now: Instant,
) -> bool {
    chase.coin == coin
        && active_symbol == coin
        && chase.current_oid.is_some()
        && !chase.has_pending_op()
        && !chase.stop_requested
        && chase.can_reprice_now(now)
        && best_px
            .and_then(|px| chase.rounded_price(px))
            .is_some_and(|(rounded_px, wire)| {
                wire != chase.current_price_wire && chase.price_moves_toward_fill(rounded_px)
            })
}
