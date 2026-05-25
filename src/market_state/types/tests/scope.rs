use super::super::{OrderBookInstance, OrderBookSymbolMode};
use super::lvl;
use crate::api::OrderBook;

mod finer;
mod replacement;

const SNAPSHOT_SCOPE: f64 = 5.0;

fn active_instance(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBookInstance {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, SNAPSHOT_SCOPE);
    inst.set_book_with_source(book(bids, asks), Some(SNAPSHOT_SCOPE));
    inst
}

fn apply_update(
    inst: &mut OrderBookInstance,
    bids: &[(f64, f64)],
    asks: &[(f64, f64)],
    source_scope: f64,
) {
    inst.apply_book_update_preserving_scope(book(bids, asks), Some(source_scope));
}

fn book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
    OrderBook {
        bids: levels(bids),
        asks: levels(asks),
    }
}

fn levels(levels: &[(f64, f64)]) -> Vec<crate::api::BookLevel> {
    levels.iter().map(|(px, sz)| lvl(*px, *sz)).collect()
}

fn bid_prices(inst: &OrderBookInstance) -> Vec<f64> {
    inst.book.bids.iter().map(|level| level.px).collect()
}

fn ask_prices(inst: &OrderBookInstance) -> Vec<f64> {
    inst.book.asks.iter().map(|level| level.px).collect()
}
