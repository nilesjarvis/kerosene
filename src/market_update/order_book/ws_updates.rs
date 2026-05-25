#[cfg(test)]
use crate::api::OrderBook;
#[cfg(test)]
use crate::helpers::positive_finite_value;
use crate::market_state::OrderBookSymbolMode;
#[cfg(test)]
use crate::signing::ChaseOrder;
#[cfg(test)]
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

#[cfg(test)]
pub(super) fn best_chase_price(book: &OrderBook, is_buy: bool) -> Option<f64> {
    let price = if is_buy {
        book.bids.first().map(|level| level.px)
    } else {
        book.asks.first().map(|level| level.px)
    };
    price.and_then(positive_finite_value)
}

#[cfg(test)]
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
        && chase.lifecycle.is_book_repriceable()
        && chase.can_reprice_now(now)
        && best_px
            .and_then(|px| chase.rounded_price(px))
            .is_some_and(|(rounded_px, wire)| {
                wire != chase.current_price_wire && chase.price_moves_toward_fill(rounded_px)
            })
}
