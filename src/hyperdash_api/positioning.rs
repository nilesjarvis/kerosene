use crate::api::CLIENT;
use reqwest::{Response, StatusCode, header::USER_AGENT};
use serde::Deserialize;

use super::errors::{hyperdash_graphql_error, hyperdash_http_error, hyperdash_missing_data_error};
use super::models::{GqlError, PerpDeltas, TickerPositions};
use super::{HYPERDASH_API_URL, KEROSENE_USER_AGENT, response_snippet};

const PERP_DELTAS_RESPONSE_MAX_BYTES: usize = 2 * 1024 * 1024;
const PERP_DELTAS_ENTRY_LIMIT: usize = 2_000;

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
    analytics: Option<GqlAnalytics>,
    #[serde(rename = "perpDeltas")]
    perp_deltas: Option<PerpDeltas>,
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

/// Fetch wallet-level position-size changes for one HyperDash perp market.
pub async fn fetch_perp_deltas(
    market: String,
    timeframe: String,
    api_key: String,
) -> Result<PerpDeltas, String> {
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
        .header("Authorization", format!("Bearer {api_key}"))
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

async fn read_perp_deltas_response_text(
    mut response: Response,
) -> Result<(StatusCode, String), String> {
    let status = response.status();
    if let Some(length) = response.content_length()
        && length > PERP_DELTAS_RESPONSE_MAX_BYTES as u64
    {
        return Err(perp_deltas_response_too_large(length));
    }

    let capacity = response
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or_default()
        .min(PERP_DELTAS_RESPONSE_MAX_BYTES);
    let mut body = Vec::with_capacity(capacity);

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Failed to read HyperDash perp deltas response: {e}"))?
    {
        append_perp_deltas_response_chunk(&mut body, chunk.as_ref())?;
    }

    String::from_utf8(body)
        .map(|text| (status, text))
        .map_err(|e| format!("Failed to decode HyperDash perp deltas response as UTF-8: {e}"))
}

fn append_perp_deltas_response_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), String> {
    let next_len = body.len().saturating_add(chunk.len());
    if next_len > PERP_DELTAS_RESPONSE_MAX_BYTES {
        return Err(perp_deltas_response_too_large(next_len as u64));
    }

    body.extend_from_slice(chunk);
    Ok(())
}

fn perp_deltas_response_too_large(byte_count: u64) -> String {
    format!(
        "HyperDash perp deltas response too large ({byte_count} bytes; max {PERP_DELTAS_RESPONSE_MAX_BYTES} bytes)"
    )
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

    if let Some(positions) = data
        .analytics
        .and_then(|analytics| analytics.perps_ticker_positions)
    {
        return Ok(positions);
    }
    if !error_messages.is_empty() {
        return Err(hyperdash_graphql_error("positioning", error_messages));
    }
    Err(hyperdash_missing_data_error("positioning"))
}

fn parse_perp_deltas_response(text: &str) -> Result<PerpDeltas, String> {
    let parsed: GqlResponse = serde_json::from_str(text).map_err(|e| {
        let snippet = response_snippet(text);
        format!("Failed to parse HyperDash perp deltas response: {e}\nResponse: {snippet}")
    })?;

    let error_messages: Vec<String> = parsed
        .errors
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.message)
        .collect();

    let Some(data) = parsed.data else {
        if !error_messages.is_empty() {
            return Err(hyperdash_graphql_error("perp deltas", error_messages));
        }
        return Err(hyperdash_missing_data_error("perp deltas"));
    };

    if let Some(mut deltas) = data.perp_deltas {
        if deltas.deltas.len() > PERP_DELTAS_ENTRY_LIMIT {
            deltas.deltas.truncate(PERP_DELTAS_ENTRY_LIMIT);
        }
        return Ok(deltas);
    }
    if !error_messages.is_empty() {
        return Err(hyperdash_graphql_error("perp deltas", error_messages));
    }
    Err(hyperdash_missing_data_error("perp deltas"))
}

#[cfg(test)]
mod tests {
    use super::{
        PERP_DELTAS_ENTRY_LIMIT, PERP_DELTAS_RESPONSE_MAX_BYTES, append_perp_deltas_response_chunk,
        parse_perp_deltas_response, parse_ticker_positions_response,
    };

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

    #[test]
    fn perp_deltas_parse_response() {
        let parsed = parse_perp_deltas_response(
            r#"{
              "data": {
                "perpDeltas": {
                  "market": "HYPE",
                  "timeframe": "15m",
                  "deltas": [{
                    "address": "0xabc0000000000000000000000000000000000000",
                    "current": -25.5,
                    "delta": 10.25
                  }]
                }
              }
            }"#,
        )
        .expect("perp deltas response should parse");

        assert_eq!(parsed.market, "HYPE");
        assert_eq!(parsed.timeframe, "15m");
        assert_eq!(parsed.deltas.len(), 1);
        assert_eq!(parsed.deltas[0].delta, 10.25);
    }

    #[test]
    fn perp_deltas_truncates_large_result_set() {
        let mut deltas = String::new();
        for index in 0..(PERP_DELTAS_ENTRY_LIMIT + 3) {
            if index > 0 {
                deltas.push(',');
            }
            deltas.push_str(&format!(
                r#"{{"address":"0x{index:040x}","current":1.0,"delta":2.0}}"#
            ));
        }

        let payload = format!(
            r#"{{"data":{{"perpDeltas":{{"market":"HYPE","timeframe":"15m","deltas":[{}]}}}}}}"#,
            deltas
        );

        let parsed =
            parse_perp_deltas_response(&payload).expect("perp deltas response should parse");

        assert_eq!(parsed.deltas.len(), PERP_DELTAS_ENTRY_LIMIT);
    }

    #[test]
    fn perp_deltas_response_chunk_cap_rejects_oversized_body_before_append() {
        let mut body = vec![b'a'; PERP_DELTAS_RESPONSE_MAX_BYTES - 1];
        let err = append_perp_deltas_response_chunk(&mut body, b"bb")
            .expect_err("oversized response body should be rejected");

        assert_eq!(body.len(), PERP_DELTAS_RESPONSE_MAX_BYTES - 1);
        assert!(err.contains("HyperDash perp deltas response too large"));
        assert!(err.contains(&PERP_DELTAS_RESPONSE_MAX_BYTES.to_string()));
    }

    #[test]
    fn perp_deltas_response_chunk_cap_accepts_exact_limit() {
        let mut body = vec![b'a'; PERP_DELTAS_RESPONSE_MAX_BYTES - 1];

        append_perp_deltas_response_chunk(&mut body, b"b")
            .expect("response body at exact cap should be accepted");

        assert_eq!(body.len(), PERP_DELTAS_RESPONSE_MAX_BYTES);
    }

    #[test]
    fn perp_deltas_reports_graphql_errors_without_data() {
        let error =
            parse_perp_deltas_response(r#"{"errors":[{"message":"invalid api key"}],"data":null}"#)
                .expect_err("graphql error should be surfaced");

        assert!(error.contains("authentication failed"));
    }
}
