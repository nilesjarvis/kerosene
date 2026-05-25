use crate::account::OpenOrder;
use crate::helpers;

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// User Order Levels
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(in crate::market_views) struct UserOrderBookLevels {
    pub(super) bids: HashSet<i64>,
    pub(super) asks: HashSet<i64>,
}

impl UserOrderBookLevels {
    pub(super) fn from_orders(orders: &[OpenOrder], symbol: &str, tick: f64) -> Self {
        if symbol.trim().is_empty() || !helpers::valid_book_tick_size(tick) {
            return Self::default();
        }

        let mut levels = Self::default();
        for order in orders.iter().filter(|order| order.coin == symbol) {
            let Some(is_bid) = order_side_is_bid(&order.side) else {
                continue;
            };
            let Some(price) = parse_order_price(&order.limit_px) else {
                continue;
            };
            let Some(key) = order_price_bucket_key(price, tick, is_bid) else {
                continue;
            };
            if is_bid {
                levels.bids.insert(key);
            } else {
                levels.asks.insert(key);
            }
        }
        levels
    }

    pub(super) fn has_bid_at_price(&self, price: f64, tick: f64) -> bool {
        displayed_price_key(price, tick).is_some_and(|key| self.bids.contains(&key))
    }

    pub(super) fn has_ask_at_price(&self, price: f64, tick: f64) -> bool {
        displayed_price_key(price, tick).is_some_and(|key| self.asks.contains(&key))
    }
}

fn order_side_is_bid(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

fn parse_order_price(value: &str) -> Option<f64> {
    helpers::parse_positive_finite_number(value)
}

fn order_price_bucket_key(price: f64, tick: f64, is_bid: bool) -> Option<i64> {
    if !helpers::valid_book_tick_size(tick) {
        return None;
    }
    let price = helpers::positive_finite_value(price)?;
    let scaled = price / tick;
    let scaled = helpers::finite_value(scaled)?;
    Some(if is_bid {
        scaled.floor() as i64
    } else {
        scaled.ceil() as i64
    })
}

fn displayed_price_key(price: f64, tick: f64) -> Option<i64> {
    if !helpers::valid_book_tick_size(tick) {
        return None;
    }
    let price = helpers::positive_finite_value(price)?;
    let scaled = price / tick;
    helpers::finite_value(scaled).map(|scaled| scaled.round() as i64)
}
