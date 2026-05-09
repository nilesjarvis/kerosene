use crate::api::CLIENT;
use reqwest::header::USER_AGENT;
use serde::Deserialize;

mod buckets;

use super::errors::{hyperdash_graphql_error, hyperdash_http_error, hyperdash_missing_data_error};
use super::models::{GqlError, LiquidationEntry, LiquidationLevel};
use super::{HYPERDASH_API_URL, KEROSENE_USER_AGENT, response_snippet};
pub use buckets::bucket_liquidations;

// ---------------------------------------------------------------------------
// HyperDash Current Liquidation Levels
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GqlResponse {
    data: Option<GqlData>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Debug, Deserialize)]
struct GqlData {
    analytics: GqlAnalytics,
}

#[derive(Debug, Deserialize)]
struct GqlAnalytics {
    #[serde(rename = "currentLiquidationLevel")]
    current_liquidation_level: GqlLiquidationLevel,
}

#[derive(Debug, Deserialize)]
struct GqlLiquidationLevel {
    coin: String,
    min: f64,
    max: f64,
    liquidations: Vec<LiquidationEntry>,
    #[serde(rename = "totalAmount")]
    total_amount: f64,
}

/// Fetch liquidation levels for a coin from the HyperDash GraphQL API.
pub async fn fetch_liquidation_levels(
    coin: String,
    min: f64,
    max: f64,
    api_key: String,
) -> Result<LiquidationLevel, String> {
    let query = r#"query GetCurrentLiquidationLevel($coin: String!, $min: Float, $max: Float) {
  analytics {
    currentLiquidationLevel(coin: $coin, min: $min, max: $max) {
      coin
      min
      max
      liquidations {
        amount
        price
      }
      totalAmount
    }
  }
}"#;

    let body = serde_json::json!({
        "operationName": "GetCurrentLiquidationLevel",
        "variables": { "coin": coin, "min": min, "max": max },
        "query": query,
    });

    let response = CLIENT
        .clone()
        .post(HYPERDASH_API_URL)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HyperDash request failed: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read HyperDash response: {e}"))?;

    if !status.is_success() {
        return Err(hyperdash_http_error("liquidation levels", status, &text));
    }

    let parsed: GqlResponse = serde_json::from_str(&text).map_err(|e| {
        let snippet = response_snippet(&text);
        format!("Failed to parse HyperDash response: {e}\nResponse: {snippet}")
    })?;

    let data = match parsed.data {
        Some(data) => data,
        None => {
            if let Some(errors) = parsed.errors {
                let messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
                return Err(hyperdash_graphql_error("liquidation levels", messages));
            }
            return Err(hyperdash_missing_data_error("liquidation levels"));
        }
    };
    let liq = data.analytics.current_liquidation_level;

    Ok(LiquidationLevel {
        coin: liq.coin,
        min: liq.min,
        max: liq.max,
        liquidations: liq.liquidations,
        total_amount: liq.total_amount,
    })
}
