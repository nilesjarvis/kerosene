use crate::ws::TrackedTradeEvent;

use super::super::super::TRACKED_TRADE_AGGREGATION_WINDOW_MS;
use super::intent::TrackedTradeIntent;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Tracked Trade Row Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct TrackedTradeFeedRow {
    pub(crate) address: String,
    pub(crate) coin: String,
    pub(crate) is_buy: bool,
    pub(crate) first_time_ms: u64,
    pub(crate) last_time_ms: u64,
    pub(crate) size: f64,
    pub(crate) notional: f64,
    pub(crate) avg_price: f64,
    pub(crate) closed_pnl: f64,
    pub(crate) fee: f64,
    pub(crate) fee_token: String,
    pub(crate) dir: String,
    pub(crate) fill_count: usize,
    pub(crate) start_position: Option<f64>,
    pub(crate) intent: TrackedTradeIntent,
    pub(crate) hash: String,
    pub(crate) oid: Option<u64>,
}

impl TrackedTradeFeedRow {
    pub(crate) fn from_event(trade: &TrackedTradeEvent) -> Self {
        let notional = trade.price * trade.size;
        let signed_size = if trade.is_buy {
            trade.size
        } else {
            -trade.size
        };
        Self {
            address: trade.address.clone(),
            coin: trade.coin.clone(),
            is_buy: trade.is_buy,
            first_time_ms: trade.time_ms,
            last_time_ms: trade.time_ms,
            size: trade.size,
            notional,
            avg_price: trade.price,
            closed_pnl: trade.closed_pnl,
            fee: trade.fee,
            fee_token: trade.fee_token.clone(),
            dir: trade.dir.clone(),
            fill_count: 1,
            start_position: trade.start_position,
            intent: TrackedTradeIntent::from_positions(trade.start_position, signed_size),
            hash: trade.hash.clone(),
            oid: trade.oid,
        }
    }

    pub(crate) fn can_merge(&self, trade: &TrackedTradeEvent) -> bool {
        if self.oid.is_some() && self.oid == trade.oid {
            return true;
        }
        if self.oid.is_none() && !self.hash.trim().is_empty() && self.hash == trade.hash {
            return true;
        }

        self.first_time_ms
            .min(trade.time_ms)
            .abs_diff(self.last_time_ms.max(trade.time_ms))
            <= TRACKED_TRADE_AGGREGATION_WINDOW_MS
    }

    pub(crate) fn add_event(&mut self, trade: &TrackedTradeEvent) {
        let event_notional = trade.price * trade.size;
        let event_is_earlier = trade.time_ms < self.first_time_ms;

        self.size += trade.size;
        self.notional += event_notional;
        if self.size > 0.0 {
            self.avg_price = self.notional / self.size;
        }
        self.closed_pnl += trade.closed_pnl;
        self.fee += trade.fee;
        if self.fee_token != trade.fee_token {
            self.fee_token = if self.fee_token.trim().is_empty() {
                trade.fee_token.clone()
            } else if trade.fee_token.trim().is_empty() {
                self.fee_token.clone()
            } else {
                "mixed".to_string()
            };
        }
        if self.dir != trade.dir {
            self.dir = "Mixed".to_string();
        }
        self.first_time_ms = self.first_time_ms.min(trade.time_ms);
        self.last_time_ms = self.last_time_ms.max(trade.time_ms);
        self.fill_count += 1;

        if event_is_earlier && trade.start_position.is_some() {
            self.start_position = trade.start_position;
        }

        let signed_size = if self.is_buy { self.size } else { -self.size };
        self.intent = TrackedTradeIntent::from_positions(self.start_position, signed_size);
    }
}
