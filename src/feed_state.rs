mod hydromancer_status;
mod liquidations;
mod tracked_trades;

pub(crate) use liquidations::{LiquidationFeedRow, liquidation_feed_scroll_id};
#[cfg(test)]
pub(crate) use tracked_trades::TrackedTradeFeedRow;
pub(crate) use tracked_trades::TrackedTradeIntent;

// ---------------------------------------------------------------------------
// Live Feed Constants
// ---------------------------------------------------------------------------

const LIQUIDATION_FEED_AGGREGATION_WINDOW_MS: u64 = 500;
const LIQUIDATION_FEED_RENDER_LIMIT: usize = 250;
const HYDROMANCER_STREAM_STALE_AFTER_MS: u64 = 75_000;
const TRACKED_TRADE_AGGREGATION_WINDOW_MS: u64 = 500;
const TRACKED_TRADE_DEDUPE_MAX: usize = 50_000;
