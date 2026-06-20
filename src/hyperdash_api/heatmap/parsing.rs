use serde::Deserialize;

use super::super::errors::{hyperdash_graphql_error, hyperdash_missing_data_error};
use super::super::models::{GqlError, HeatmapRect, LiquidationHeatmap};
use super::super::{HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS, response_snippet};

// ---------------------------------------------------------------------------
// Heatmap Response Parsing
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GqlHeatmapResponse {
    data: Option<GqlHeatmapData>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Deserialize)]
struct GqlHeatmapData {
    analytics: GqlHeatmapAnalytics,
}

#[derive(Deserialize)]
struct GqlHeatmapAnalytics {
    #[serde(rename = "liquidationLevels")]
    liquidation_levels: GqlHeatmapLevels,
}

#[derive(Deserialize)]
struct GqlHeatmapLevels {
    bands: Vec<GqlHeatmapBand>,
}

#[derive(Deserialize)]
struct GqlHeatmapBand {
    #[serde(rename = "minPrice")]
    min_price: f64,
    #[serde(rename = "maxPrice")]
    max_price: f64,
    #[serde(rename = "historicalData")]
    historical_data: Vec<GqlHeatmapCell>,
}

#[derive(Deserialize)]
struct GqlHeatmapCell {
    timestamp: String,
    #[serde(rename = "totalAmount")]
    total_amount: f64,
}

pub(super) fn parse_heatmap_response(text: &str) -> Result<LiquidationHeatmap, String> {
    let parsed: GqlHeatmapResponse = serde_json::from_str(text).map_err(|e| {
        let snippet = response_snippet(text);
        format!("Failed to parse HyperDash heatmap response: {e}\nResponse: {snippet}")
    })?;

    let data = match parsed.data {
        Some(data) => data,
        None => {
            if let Some(errors) = parsed.errors {
                let messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
                return Err(hyperdash_graphql_error("heatmap", messages));
            }
            return Err(hyperdash_missing_data_error("heatmap"));
        }
    };
    let lev = data.analytics.liquidation_levels;

    let mut timestamps_ms = Vec::new();
    for band in &lev.bands {
        for cell in &band.historical_data {
            if let Some(ts_ms) = parse_heatmap_timestamp(&cell.timestamp) {
                timestamps_ms.push(ts_ms);
            }
        }
    }
    let bucket_duration_ms = infer_heatmap_bucket_duration_ms(&timestamps_ms);

    let mut rects = Vec::new();
    let mut max_abs_usd: f64 = 0.0;

    for band in &lev.bands {
        let mid_price = (band.min_price + band.max_price) * 0.5;
        for cell in &band.historical_data {
            let Some(ts_ms) = parse_heatmap_timestamp(&cell.timestamp) else {
                continue;
            };
            let usd = cell.total_amount.abs() * mid_price;
            max_abs_usd = max_abs_usd.max(usd);
            rects.push(HeatmapRect {
                timestamp_ms: ts_ms,
                duration_ms: bucket_duration_ms,
                price_lo: band.min_price,
                price_hi: band.max_price,
                amount_coins: cell.total_amount,
                amount_usd: if cell.total_amount >= 0.0 { usd } else { -usd },
            });
        }
    }

    Ok(LiquidationHeatmap { rects, max_abs_usd })
}

/// Parse a "YYYY-MM-DD HH:MM:SS" UTC string to epoch milliseconds.
/// The format from HyperDash is always this fixed format.
pub(super) fn parse_heatmap_timestamp(s: &str) -> Option<u64> {
    if s.len() < 19 {
        return None;
    }
    let year: u64 = s[0..4].parse().ok()?;
    let month: u64 = s[5..7].parse().ok()?;
    let day: u64 = s[8..10].parse().ok()?;
    let hour: u64 = s[11..13].parse().ok()?;
    let min: u64 = s[14..16].parse().ok()?;
    let sec: u64 = s[17..19].parse().ok()?;

    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 12 } else { month };
    let days =
        (365 * y) + (y / 4) - (y / 100) + (y / 400) + ((153 * (m - 3) + 2) / 5) + day - 719469;
    let epoch_secs = days * 86400 + hour * 3600 + min * 60 + sec;
    Some(epoch_secs * 1000)
}

pub(super) fn infer_heatmap_bucket_duration_ms(timestamps_ms: &[u64]) -> u64 {
    let default_ms = HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS * 1000;
    if timestamps_ms.len() < 2 {
        return default_ms;
    }

    let mut sorted = timestamps_ms.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted
        .windows(2)
        .filter_map(|pair| {
            let gap = pair[1].saturating_sub(pair[0]);
            (gap > 0).then_some(gap)
        })
        .min()
        .unwrap_or(default_ms)
}
