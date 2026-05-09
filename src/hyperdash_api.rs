mod errors;
mod heatmap;
mod liquidation_levels;
mod models;

pub use heatmap::{fetch_liquidation_heatmap, normalize_heatmap_time_range};
pub use liquidation_levels::{bucket_liquidations, fetch_liquidation_levels_at};
pub use models::{
    HeatmapFetchParams, HeatmapRect, LiquidationBucket, LiquidationHeatmap, LiquidationLevel,
};

// ---------------------------------------------------------------------------
// HyperDash GraphQL API
// ---------------------------------------------------------------------------

const KEROSENE_USER_AGENT: &str = concat!("Kerosene/", env!("CARGO_PKG_VERSION"));

pub const HYPERDASH_API_URL: &str = "https://api.hyperdash.com/graphql";
pub const HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS: u64 = 60 * 60;
pub const HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS: u64 = 7 * 24 * 60 * 60;

fn response_snippet(text: &str) -> String {
    let mut snippet: String = text.chars().take(200).collect();
    if text.chars().count() > 200 {
        snippet.push_str("...");
    }
    snippet
}
