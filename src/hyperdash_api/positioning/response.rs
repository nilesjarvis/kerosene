use reqwest::{Response, StatusCode};
use serde::Deserialize;

use super::super::errors::{hyperdash_graphql_error, hyperdash_missing_data_error};
use super::super::models::{GqlError, PerpDeltas, TickerPositions};
use super::super::response_snippet;

// ---------------------------------------------------------------------------
// HyperDash Positioning Responses
// ---------------------------------------------------------------------------

pub(super) const PERP_DELTAS_RESPONSE_MAX_BYTES: usize = 2 * 1024 * 1024;
pub(super) const PERP_DELTAS_ENTRY_LIMIT: usize = 2_000;

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

pub(super) async fn read_perp_deltas_response_text(
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

pub(super) fn append_perp_deltas_response_chunk(
    body: &mut Vec<u8>,
    chunk: &[u8],
) -> Result<(), String> {
    let next_len = body.len().saturating_add(chunk.len());
    if next_len > PERP_DELTAS_RESPONSE_MAX_BYTES {
        return Err(perp_deltas_response_too_large(next_len as u64));
    }

    body.extend_from_slice(chunk);
    Ok(())
}

pub(super) fn parse_ticker_positions_response(text: &str) -> Result<TickerPositions, String> {
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

pub(super) fn parse_perp_deltas_response(text: &str) -> Result<PerpDeltas, String> {
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

fn perp_deltas_response_too_large(byte_count: u64) -> String {
    format!(
        concat!(
            "HyperDash perp deltas response too large ",
            "({} bytes; max {} bytes)"
        ),
        byte_count, PERP_DELTAS_RESPONSE_MAX_BYTES
    )
}
