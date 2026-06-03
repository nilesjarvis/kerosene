mod errors;
mod heatmap;
mod liquidation_levels;
mod models;
mod positioning;

use crate::helpers::response_snippet;

pub use heatmap::{fetch_liquidation_heatmap, normalize_heatmap_time_range};
pub use liquidation_levels::{bucket_liquidations, fetch_liquidation_levels_at};
pub use models::{
    HeatmapFetchParams, HeatmapRect, LiquidationBucket, LiquidationHeatmap, LiquidationLevel,
    PerpDeltaEntry, PerpDeltas, TickerPositionEntry, TickerPositions,
};
pub use positioning::{fetch_perp_deltas, fetch_ticker_positions};

// ---------------------------------------------------------------------------
// HyperDash GraphQL API
// ---------------------------------------------------------------------------

const KEROSENE_USER_AGENT: &str = concat!("Kerosene/", env!("CARGO_PKG_VERSION"));

pub const HYPERDASH_API_URL: &str = "https://api.hyperdash.com/graphql";
pub const HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS: u64 = 60 * 60;
pub const HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS: u64 = 7 * 24 * 60 * 60;
