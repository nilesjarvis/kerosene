use crate::ws::TrackedTradeEvent;

use std::fmt;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Tracked Trade Aggregation Key
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::feed_state::tracked_trades) enum TrackedTradeAggregationKey<'a> {
    Order {
        address: &'a str,
        coin: &'a str,
        is_buy: bool,
        oid: u64,
    },
    Hash {
        address: &'a str,
        coin: &'a str,
        is_buy: bool,
        hash: &'a str,
    },
    TimeWindow {
        address: &'a str,
        coin: &'a str,
        is_buy: bool,
        dir: &'a str,
    },
}

impl fmt::Debug for TrackedTradeAggregationKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Order { is_buy, .. } => f
                .debug_struct("Order")
                .field("identity", &"<redacted>")
                .field("is_buy", is_buy)
                .finish(),
            Self::Hash { is_buy, .. } => f
                .debug_struct("Hash")
                .field("identity", &"<redacted>")
                .field("is_buy", is_buy)
                .finish(),
            Self::TimeWindow { is_buy, .. } => f
                .debug_struct("TimeWindow")
                .field("identity", &"<redacted>")
                .field("is_buy", is_buy)
                .finish(),
        }
    }
}

impl<'a> TrackedTradeAggregationKey<'a> {
    pub(in crate::feed_state::tracked_trades) fn from_event(trade: &'a TrackedTradeEvent) -> Self {
        if let Some(oid) = trade.oid {
            Self::Order {
                address: &trade.address,
                coin: &trade.coin,
                is_buy: trade.is_buy,
                oid,
            }
        } else if !trade.hash.trim().is_empty() {
            Self::Hash {
                address: &trade.address,
                coin: &trade.coin,
                is_buy: trade.is_buy,
                hash: &trade.hash,
            }
        } else {
            Self::TimeWindow {
                address: &trade.address,
                coin: &trade.coin,
                is_buy: trade.is_buy,
                dir: &trade.dir,
            }
        }
    }
}
