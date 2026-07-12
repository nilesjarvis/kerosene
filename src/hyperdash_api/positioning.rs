use crate::api::CLIENT;
use reqwest::header::USER_AGENT;
use zeroize::Zeroizing;

use self::response::{
    parse_perp_deltas_response, parse_ticker_positions_response, read_perp_deltas_response_text,
};
use super::errors::hyperdash_http_error;
use super::models::{PerpDeltas, TickerPositions};
use super::{HYPERDASH_API_URL, KEROSENE_USER_AGENT};

mod response;

// ---------------------------------------------------------------------------
// HyperDash Ticker Positioning
// ---------------------------------------------------------------------------

/// Fetch wallet-level perp positioning for one HyperDash ticker.
#[allow(clippy::too_many_arguments)]
pub async fn fetch_ticker_positions(
    coin: String,
    limit: u32,
    offset: u32,
    side: String,
    sort_field: String,
    sort_order: String,
    entry_min: Option<f64>,
    entry_max: Option<f64>,
    api_key: Zeroizing<String>,
) -> Result<TickerPositions, String> {
    let api_key = Zeroizing::new(api_key.trim().to_string());
    let query = r#"query GetTickerPositions(
  $coin: String!,
  $limit: Int,
  $offset: Int,
  $side: String,
  $filters: PerpsFilterInput,
  $sortBy: PerpsTickerSortInput
) {
  analytics {
    perpsTickerPositions(
      coin: $coin
      limit: $limit
      offset: $offset
      side: $side
      filters: $filters
      sortBy: $sortBy
    ) {
      coin
      positions {
        address
        displayName
        label
        tag
        verified
        copyScore
        size
        notionalSize
        entryPrice
        liquidationPrice
        unrealizedPnl
        fundingPnl
        accountValue
      }
      totalLongNotional
      totalShortNotional
      totalNotional
      longCount
      shortCount
      totalCount
      hasMore
      timestamp
    }
  }
}"#;

    let mut variables = serde_json::json!({
        "coin": coin,
        "limit": limit,
        "offset": offset,
        "side": side,
        "sortBy": {
            "field": sort_field,
            "order": sort_order,
        },
    });
    if let Some(filters) = positioning_entry_range_filters(entry_min, entry_max)
        && let Some(variables) = variables.as_object_mut()
    {
        variables.insert("filters".to_string(), filters);
    }

    let body = serde_json::json!({
        "operationName": "GetTickerPositions",
        "variables": variables,
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
        .map_err(|e| format!("HyperDash positioning request failed: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read HyperDash positioning response: {e}"))?;

    if !status.is_success() {
        return Err(hyperdash_http_error("positioning", status, &text));
    }

    parse_ticker_positions_response(&text)
}

fn positioning_entry_range_filters(
    entry_min: Option<f64>,
    entry_max: Option<f64>,
) -> Option<serde_json::Value> {
    let mut filters = serde_json::Map::new();
    if let Some(entry_min) = entry_min {
        filters.insert("minEntry".to_string(), serde_json::json!(entry_min));
    }
    if let Some(entry_max) = entry_max {
        filters.insert("maxEntry".to_string(), serde_json::json!(entry_max));
    }
    (!filters.is_empty()).then_some(serde_json::Value::Object(filters))
}

/// Fetch wallet-level position-size changes for one HyperDash perp market.
pub async fn fetch_perp_deltas(
    market: String,
    timeframe: String,
    api_key: Zeroizing<String>,
) -> Result<PerpDeltas, String> {
    let api_key = Zeroizing::new(api_key.trim().to_string());
    let query = r#"query GetPerpDeltas($market: String!, $timeframe: DeltaTimeframe!) {
  perpDeltas(market: $market, timeframe: $timeframe) {
    market
    timeframe
    deltas {
      address
      current
      delta
    }
  }
}"#;

    let body = serde_json::json!({
        "operationName": "GetPerpDeltas",
        "variables": {
            "market": market,
            "timeframe": timeframe,
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
        .map_err(|e| format!("HyperDash perp deltas request failed: {e}"))?;

    let (status, text) = read_perp_deltas_response_text(response).await?;

    if !status.is_success() {
        return Err(hyperdash_http_error("perp deltas", status, &text));
    }

    parse_perp_deltas_response(&text)
}

#[cfg(test)]
mod tests;
