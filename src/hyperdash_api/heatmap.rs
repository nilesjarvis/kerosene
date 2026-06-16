use crate::api::CLIENT;
use reqwest::header::USER_AGENT;
use zeroize::Zeroizing;

use self::parsing::parse_heatmap_response;
#[cfg(test)]
use self::parsing::{infer_heatmap_bucket_duration_ms, parse_heatmap_timestamp};
use super::errors::hyperdash_http_error;
use super::models::LiquidationHeatmap;
use super::{
    HYPERDASH_API_URL, HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS, HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS,
    KEROSENE_USER_AGENT,
};

mod parsing;

// ---------------------------------------------------------------------------
// HyperDash Historical Liquidation Heatmap
// ---------------------------------------------------------------------------

pub fn normalize_heatmap_time_range(
    start_time: u64,
    end_time: u64,
    now_time: u64,
) -> Option<(u64, u64)> {
    if now_time == 0 {
        return None;
    }

    let latest_available_start = now_time.saturating_sub(HYPERDASH_HEATMAP_MAX_LOOKBACK_SECS);
    let mut end = if end_time == 0 {
        now_time
    } else {
        end_time.min(now_time)
    };

    if end < latest_available_start {
        return None;
    }

    let requested_start = start_time.min(end);
    let mut start = requested_start.max(latest_available_start);
    let minimum_start = end.saturating_sub(HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS);
    if start > minimum_start {
        start = minimum_start.max(latest_available_start);
    }

    if end <= start {
        end = (start + HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS).min(now_time);
    }

    if end <= start {
        None
    } else {
        Some((start, end))
    }
}

/// Fetch historical liquidation heatmap data from the HyperDash GraphQL API.
/// `start_time` and `end_time` are in epoch seconds.
pub async fn fetch_liquidation_heatmap(
    coin: String,
    min_price: f64,
    max_price: f64,
    start_time: u64,
    end_time: u64,
    api_key: Zeroizing<String>,
) -> Result<LiquidationHeatmap, String> {
    let api_key = Zeroizing::new(api_key.trim().to_string());
    let query = r#"query GetLiquidationLevels($coin: String!, $minPrice: Float!, $maxPrice: Float!, $startTime: Float!, $endTime: Float) {
  analytics {
    liquidationLevels(
      coin: $coin
      minPrice: $minPrice
      maxPrice: $maxPrice
      startTime: $startTime
      endTime: $endTime
    ) {
      bands {
        minPrice
        maxPrice
        historicalData {
          timestamp
          totalAmount
        }
      }
    }
  }
}"#;

    let body = serde_json::json!({
        "operationName": "GetLiquidationLevels",
        "variables": {
            "coin": coin,
            "minPrice": min_price,
            "maxPrice": max_price,
            "startTime": start_time,
            "endTime": end_time,
        },
        "query": query,
    });

    let response = CLIENT
        .clone()
        .post(HYPERDASH_API_URL)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .bearer_auth(api_key.as_str())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HyperDash heatmap request failed: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read HyperDash heatmap response: {e}"))?;

    if !status.is_success() {
        return Err(hyperdash_http_error("heatmap", status, &text));
    }

    parse_heatmap_response(&text)
}

#[cfg(test)]
mod tests;
