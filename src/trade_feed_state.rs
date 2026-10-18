use crate::config::OrderBookSymbolModeConfig;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) type TradeFeedId = u64;
pub(crate) const TRADE_FEED_CAPACITY: usize = 500;
const SEEN_CAPACITY: usize = 4096;

/// Public execution data only; participant addresses and hashes are not retained.
#[derive(Debug, Clone)]
pub(crate) struct TradePrint {
    pub(crate) time_ms: u64,
    pub(crate) tid: u64,
    pub(crate) is_buy: bool,
    pub(crate) price: String,
    pub(crate) size: String,
    pub(crate) value: f64,
}

#[derive(Debug, Clone)]
pub(crate) enum TradeFeedEvent {
    Trades(Vec<TradePrint>),
    Subscribed,
    Reconnecting,
    Lagged,
    Unavailable,
}

#[derive(Default)]
pub(crate) struct TradeFeedsState {
    pub(crate) instances: HashMap<TradeFeedId, TradeFeedInstance>,
    pub(crate) next_id: TradeFeedId,
    /// Never reset on layout restoration: old streams may still have queued messages.
    pub(crate) generation: u64,
}

pub(crate) struct TradeFeedInstance {
    pub(crate) mode: OrderBookSymbolModeConfig,
    pub(crate) symbol: Option<String>,
    pub(crate) generation: u64,
    pub(crate) picker_open: bool,
    pub(crate) search: String,
    pub(crate) trades: VecDeque<TradePrint>,
    pub(crate) status: &'static str,
    pub(crate) has_gap: bool,
    seen: HashSet<(u64, u64)>,
    seen_order: VecDeque<(u64, u64)>,
}

impl TradeFeedInstance {
    pub(crate) fn new(mode: OrderBookSymbolModeConfig) -> Self {
        Self {
            mode,
            symbol: None,
            generation: 0,
            picker_open: false,
            search: String::new(),
            trades: VecDeque::new(),
            status: "Waiting for trades",
            has_gap: false,
            seen: HashSet::new(),
            seen_order: VecDeque::new(),
        }
    }

    pub(crate) fn reset(&mut self, symbol: Option<String>, generation: u64) {
        self.symbol = symbol;
        self.generation = generation;
        self.trades.clear();
        self.seen.clear();
        self.seen_order.clear();
        self.status = "Waiting for trades";
        self.has_gap = false;
    }

    pub(crate) fn apply(&mut self, event: TradeFeedEvent) {
        match event {
            TradeFeedEvent::Trades(trades) => {
                // A long older replay can evict dedupe-cache keys for newer rows
                // that are still displayed. Those rows must remain unique too.
                let retained: HashSet<_> = self
                    .trades
                    .iter()
                    .map(|trade| (trade.time_ms, trade.tid))
                    .collect();
                for trade in trades {
                    let key = (trade.time_ms, trade.tid);
                    if !retained.contains(&key) && self.seen.insert(key) {
                        self.seen_order.push_back(key);
                        self.trades.push_front(trade);
                    }
                }
                // Snapshots can arrive out of order. Stable time sorting preserves reverse
                // receipt order within a block; tid is a hash, not a sequence number.
                self.trades
                    .make_contiguous()
                    .sort_by_key(|trade| Reverse(trade.time_ms));
                self.trades.truncate(TRADE_FEED_CAPACITY);
                while self.seen_order.len() > SEEN_CAPACITY {
                    if let Some(key) = self.seen_order.pop_front() {
                        self.seen.remove(&key);
                    }
                }
                self.status = "Live";
            }
            TradeFeedEvent::Subscribed => self.status = "Waiting for trades",
            TradeFeedEvent::Reconnecting => {
                self.status = "Reconnecting";
                self.has_gap = true;
            }
            TradeFeedEvent::Lagged => {
                self.status = "Catching up";
                self.has_gap = true;
            }
            TradeFeedEvent::Unavailable => self.status = "Feed unavailable",
        }
    }
}

/// Relative notional keeps the emphasis useful across differently priced assets.
pub(crate) fn trade_shading(value: f64, largest: f64) -> f32 {
    if !value.is_finite() || !largest.is_finite() || value <= 0.0 || largest <= 0.0 {
        return 0.0;
    }
    0.03 + 0.27 * (value / largest).clamp(0.0, 1.0).sqrt() as f32
}

/// Exact wire identifiers precede display matches, prefixes, and title substrings.
pub(crate) fn trade_feed_symbol_rank(query: &str, key: &str, display: &str) -> Option<u8> {
    let query = query.trim().to_lowercase();
    let key = key.to_lowercase();
    let display = display.to_lowercase();
    if key == query {
        Some(0)
    } else if display == query {
        Some(1)
    } else if key.starts_with(&query) || display.starts_with(&query) {
        Some(2)
    } else if key.contains(&query) || display.contains(&query) {
        Some(3)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
