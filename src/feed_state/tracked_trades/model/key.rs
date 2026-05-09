use crate::ws::TrackedTradeEvent;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Tracked Trade Aggregation Key
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
