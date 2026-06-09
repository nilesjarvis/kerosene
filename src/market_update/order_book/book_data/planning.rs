use crate::helpers;
use crate::market_state::{OrderBookId, OrderBookSymbolMode};

// ---------------------------------------------------------------------------
// Order Book Fetch Planning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(in crate::market_update::order_book) struct OrderBookFetchPlan {
    pub(in crate::market_update::order_book) id: OrderBookId,
    pub(in crate::market_update::order_book) symbol: String,
    pub(in crate::market_update::order_book) tick_size: f64,
    pub(in crate::market_update::order_book) sigfigs: (Option<u8>, Option<u8>),
}

// REST l2Book snapshots return roughly 20 levels per side. Refresh once the
// market has moved halfway through that remembered coarse-depth window.
const ORDER_BOOK_SOURCE_LEVELS_PER_SIDE: f64 = 20.0;
const ORDER_BOOK_SCOPE_REFRESH_FRACTION: f64 = 0.5;

pub(in crate::market_update::order_book) fn plan_order_book_fetch(
    id: OrderBookId,
    mode: &OrderBookSymbolMode,
    active_symbol: &str,
    tick_size: f64,
    book_mid: f64,
    fallback_mid: Option<f64>,
    unavailable: bool,
) -> Option<OrderBookFetchPlan> {
    let symbol = match mode {
        OrderBookSymbolMode::Active => active_symbol.to_string(),
        OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
    };
    if symbol.is_empty() || unavailable {
        return None;
    }

    let mid = helpers::positive_finite_value(book_mid)
        .or_else(|| fallback_mid.and_then(helpers::positive_finite_value));
    let sigfigs = mid
        .map(|mid| helpers::compute_sigfigs(tick_size, mid))
        .unwrap_or((None, None));

    Some(OrderBookFetchPlan {
        id,
        symbol,
        tick_size,
        sigfigs,
    })
}

pub(in crate::market_update::order_book) fn order_book_needs_precision_refresh(
    selected_tick: f64,
    source_tick: Option<f64>,
    source_mid: Option<f64>,
    pending_sigfigs: Option<(Option<u8>, Option<u8>)>,
    book_loading: bool,
    mid: Option<f64>,
) -> bool {
    if book_loading {
        return false;
    }

    let Some(mid) = mid.and_then(helpers::positive_finite_value) else {
        return false;
    };
    if !saved_tick_requires_aggregated_fetch(selected_tick, mid) {
        return false;
    }

    let expected_sigfigs = helpers::compute_sigfigs(selected_tick, mid);
    if pending_sigfigs == Some(expected_sigfigs) {
        return false;
    }

    let Some(expected_source_tick) = helpers::sigfig_server_tick(expected_sigfigs, mid) else {
        return false;
    };
    if !source_tick.is_some_and(|actual| helpers::tick_sizes_match(actual, expected_source_tick)) {
        return true;
    }

    order_book_source_scope_is_stale(source_mid, source_tick, mid)
}

fn saved_tick_requires_aggregated_fetch(selected_tick: f64, mid: f64) -> bool {
    if !helpers::valid_book_tick_size(selected_tick) {
        return false;
    }
    let default_tick = helpers::default_tick_for_price(mid);
    selected_tick > default_tick && !helpers::tick_sizes_match(selected_tick, default_tick)
}

pub(in crate::market_update::order_book) fn order_book_response_matches_expected_precision(
    tick_size: f64,
    sigfigs: (Option<u8>, Option<u8>),
    mid: Option<f64>,
) -> bool {
    let Some(mid) = mid.and_then(helpers::positive_finite_value) else {
        return true;
    };
    if !saved_tick_requires_aggregated_fetch(tick_size, mid) {
        return true;
    }

    sigfigs == helpers::compute_sigfigs(tick_size, mid)
}

fn order_book_source_scope_is_stale(
    source_mid: Option<f64>,
    source_tick: Option<f64>,
    current_mid: f64,
) -> bool {
    let Some(source_mid) = source_mid.and_then(helpers::positive_finite_value) else {
        return false;
    };
    let Some(source_tick) = source_tick.and_then(helpers::positive_finite_value) else {
        return false;
    };

    let refresh_distance =
        source_tick * ORDER_BOOK_SOURCE_LEVELS_PER_SIDE * ORDER_BOOK_SCOPE_REFRESH_FRACTION;
    (current_mid - source_mid).abs() >= refresh_distance
}
