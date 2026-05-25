use super::*;
use crate::api::Candle;
use crate::chart::model::TradeMarker;

mod geometry;
mod grouping;

fn candle(open_time: u64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: 100.0,
        high: 110.0,
        low: 90.0,
        close: 105.0,
        volume: 1.0,
    }
}

fn marker(time_ms: u64, price: f64, size: f64, is_buy: bool) -> TradeMarker {
    TradeMarker {
        time_ms,
        price,
        size,
        is_buy,
    }
}

fn marker_group_for_side_or_panic(groups: &[TradeMarkerGroup], is_buy: bool) -> &TradeMarkerGroup {
    for group in groups {
        if group.is_buy == is_buy {
            return group;
        }
    }

    panic!("missing trade marker group for buy={is_buy}");
}

fn marker_clamp_bounds_or_panic(price_height: f32) -> (f32, f32) {
    match trade_marker_clamp_bounds(price_height) {
        Some(bounds) => bounds,
        None => panic!("valid marker clamp bounds"),
    }
}

fn marker_anchor_or_panic(candle: &Candle, is_buy: bool, price_to_y: &impl Fn(f64) -> f32) -> f32 {
    match trade_marker_anchor_y(candle, is_buy, price_to_y) {
        Some(anchor) => anchor,
        None => panic!("valid marker anchor"),
    }
}
