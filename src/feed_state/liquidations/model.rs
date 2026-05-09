use crate::ws::LiquidationEvent;

use super::super::LIQUIDATION_FEED_AGGREGATION_WINDOW_MS;

// ---------------------------------------------------------------------------
// Liquidation Feed Rows
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct LiquidationAggregationKey<'a> {
    coin: &'a str,
    is_buy: bool,
    method: &'a str,
    liquidated_user: &'a str,
    exact_time_ms: Option<u64>,
    exact_tx_index: Option<u64>,
}

impl<'a> LiquidationAggregationKey<'a> {
    pub(super) fn from_event(liq: &'a LiquidationEvent) -> Self {
        let exact_only = liq.liquidated_user.trim().is_empty();

        Self {
            coin: &liq.coin,
            is_buy: liq.is_buy,
            method: &liq.method,
            liquidated_user: &liq.liquidated_user,
            exact_time_ms: exact_only.then_some(liq.time_ms),
            exact_tx_index: exact_only.then_some(liq.tx_index),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LiquidationFeedRow {
    pub(crate) coin: String,
    pub(crate) price: f64,
    pub(crate) size: f64,
    pub(crate) is_buy: bool,
    pub(crate) time_ms: u64,
    pub(crate) method: String,
    pub(crate) liquidated_user: String,
    pub(crate) tx_index: u64,
    pub(crate) fill_count: usize,
    pub(crate) notional: f64,
}

impl LiquidationFeedRow {
    pub(super) fn from_event(liq: &LiquidationEvent) -> Self {
        Self {
            coin: liq.coin.clone(),
            price: liq.price,
            size: liq.size,
            is_buy: liq.is_buy,
            time_ms: liq.time_ms,
            method: liq.method.clone(),
            liquidated_user: liq.liquidated_user.clone(),
            tx_index: liq.tx_index,
            fill_count: 1,
            notional: liq.size * liq.price,
        }
    }

    pub(super) fn can_merge(&self, liq: &LiquidationEvent) -> bool {
        if self.liquidated_user.trim().is_empty() {
            return self.time_ms == liq.time_ms && self.tx_index == liq.tx_index;
        }

        self.time_ms.abs_diff(liq.time_ms) <= LIQUIDATION_FEED_AGGREGATION_WINDOW_MS
    }

    pub(super) fn add_event(&mut self, liq: &LiquidationEvent) {
        let event_notional = liq.size * liq.price;

        self.size += liq.size;
        self.notional += event_notional;
        self.time_ms = self.time_ms.max(liq.time_ms);
        self.fill_count += 1;

        if self.size > 0.0 {
            self.price = self.notional / self.size;
        }
    }
}
