use crate::api::CLIENT;
use reqwest::header::USER_AGENT;
use serde::Deserialize;

use super::errors::{hyperdash_graphql_error, hyperdash_http_error, hyperdash_missing_data_error};
use super::models::{GqlError, TickerPositions};
use super::{HYPERDASH_API_URL, KEROSENE_USER_AGENT, response_snippet};

// ---------------------------------------------------------------------------
// HyperDash Ticker Positioning
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
    #[serde(rename = "perpsTickerPositions")]
    perps_ticker_positions: Option<TickerPositions>,
}

/// Fetch wallet-level perp positioning for one HyperDash ticker.
pub async fn fetch_ticker_positions(
    coin: String,
    limit: u32,
    offset: u32,
    side: String,
    sort_field: String,
    sort_order: String,
    api_key: String,
) -> Result<TickerPositions, String> {
    let query = r#"query GetTickerPositions($coin: String!, $limit: Int, $offset: Int, $side: String, $filters: PerpsFilterInput, $sortBy: PerpsTickerSortInput) {
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

    let body = serde_json::json!({
        "operationName": "GetTickerPositions",
        "variables": {
            "coin": coin,
            "limit": limit,
            "offset": offset,
            "side": side,
            "sortBy": {
                "field": sort_field,
                "order": sort_order,
            },
        },
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

fn parse_ticker_positions_response(text: &str) -> Result<TickerPositions, String> {
    let parsed: GqlResponse = serde_json::from_str(text).map_err(|e| {
        let snippet = response_snippet(text);
        format!("Failed to parse HyperDash positioning response: {e}\nResponse: {snippet}")
    })?;

    let error_messages: Vec<String> = parsed
        .errors
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.message)
        .collect();

    let Some(data) = parsed.data else {
        if !error_messages.is_empty() {
            return Err(hyperdash_graphql_error("positioning", error_messages));
        }
        return Err(hyperdash_missing_data_error("positioning"));
    };

    if let Some(positions) = data.analytics.perps_ticker_positions {
        return Ok(positions);
    }
    if !error_messages.is_empty() {
        return Err(hyperdash_graphql_error("positioning", error_messages));
    }
    Err(hyperdash_missing_data_error("positioning"))
}

#[cfg(test)]
mod tests {
    use super::parse_ticker_positions_response;

    #[test]
    fn ticker_positions_parse_nullable_identity_and_liquidation_fields() {
        let parsed = parse_ticker_positions_response(
            r#"{
              "data": {
                "analytics": {
                  "perpsTickerPositions": {
                    "coin": "HYPE",
                    "positions": [{
                      "address": "0xabc0000000000000000000000000000000000000",
                      "displayName": null,
                      "label": "Whale",
                      "tag": "swing",
                      "verified": null,
                      "copyScore": 61.5,
                      "size": 12.5,
                      "notionalSize": 500.25,
                      "entryPrice": 30.0,
                      "liquidationPrice": null,
                      "unrealizedPnl": 125.75,
                      "fundingPnl": -4.5,
                      "accountValue": 1000.0
                    }],
                    "totalLongNotional": 600.0,
                    "totalShortNotional": 400.0,
                    "totalNotional": 1000.0,
                    "longCount": 3,
                    "shortCount": 2,
                    "totalCount": 5,
                    "hasMore": true,
                    "timestamp": "2026-05-18T11:52:39.585Z"
                  }
                }
              }
            }"#,
        )
        .expect("positioning response should parse");

        assert_eq!(parsed.coin, "HYPE");
        assert_eq!(parsed.total_count, 5);
        assert!(parsed.has_more);
        assert_eq!(parsed.positions.len(), 1);
        assert_eq!(parsed.positions[0].label.as_deref(), Some("Whale"));
        assert_eq!(parsed.positions[0].liquidation_price, None);
    }

    #[test]
    fn ticker_positions_reports_graphql_errors_without_data() {
        let error = parse_ticker_positions_response(
            r#"{"errors":[{"message":"invalid api key"}],"data":null}"#,
        )
        .expect_err("graphql error should be surfaced");

        assert!(error.contains("authentication failed"));
    }

    #[test]
    fn ticker_positions_reports_graphql_errors_for_missing_partial_field() {
        let error = parse_ticker_positions_response(
            r#"{
              "data": {"analytics": {"perpsTickerPositions": null}},
              "errors": [{"message": "coin not found"}]
            }"#,
        )
        .expect_err("partial graphql error should be surfaced");

        assert!(error.contains("coin not found"));
    }
}
