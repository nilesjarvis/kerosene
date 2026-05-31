use super::*;
use crate::api::{BookLevel, OrderBook};

mod aggregation;
mod cache;
mod mid_price;
mod scope;

fn lvl(px: f64, sz: f64) -> BookLevel {
    BookLevel { px, sz }
}

fn book_at_mid(mid: f64) -> OrderBook {
    OrderBook {
        bids: vec![lvl(mid - 0.5, 1.0)],
        asks: vec![lvl(mid + 0.5, 1.0)],
    }
}

#[test]
fn symbol_search_sort_mode_config_values_round_trip() {
    for mode in SymbolSearchSortMode::ALL {
        assert_eq!(
            SymbolSearchSortMode::from_config_str(mode.config_value()),
            mode
        );
    }
    assert_eq!(
        SymbolSearchSortMode::from_config_str("unknown"),
        SymbolSearchSortMode::Relevance
    );
}

#[test]
fn order_book_spread_chart_height_is_clamped_at_state_boundary() {
    let mut instance = OrderBookInstance::new(1, OrderBookSymbolMode::Active, 0.01);

    instance.set_spread_chart_height(10.0);
    assert_eq!(
        instance.spread_chart_height,
        MIN_ORDER_BOOK_SPREAD_CHART_HEIGHT
    );

    instance.set_spread_chart_height(2_000.0);
    assert_eq!(
        instance.spread_chart_height,
        MAX_ORDER_BOOK_SPREAD_CHART_HEIGHT
    );

    instance.set_spread_chart_height(f32::NAN);
    assert_eq!(
        instance.spread_chart_height,
        DEFAULT_ORDER_BOOK_SPREAD_CHART_HEIGHT
    );
}
