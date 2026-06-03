use super::{Candle, normalize_candles};
use crate::helpers::response_snippet;
use reqwest::StatusCode;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Candle Response Parsing
// ---------------------------------------------------------------------------

pub(super) fn parse_candle_response(
    status: StatusCode,
    content_type: Option<&str>,
    text: &str,
) -> Result<Vec<Candle>, String> {
    let trimmed = text.trim();
    if !status.is_success() {
        return Err(format!(
            "Candle request failed (HTTP {}): {}",
            status,
            response_snippet(text)
        ));
    }

    if content_type.is_some_and(|value| !value.to_ascii_lowercase().contains("json"))
        && !trimmed.starts_with('[')
        && trimmed != "null"
    {
        return Err(format!(
            "Candle request returned {} instead of JSON: {}",
            content_type.unwrap_or("unknown content"),
            response_snippet(text)
        ));
    }

    // The API returns `null` for coins with no candle data (e.g. newly
    // listed assets, delisted pairs). Treat null as an empty list.
    if trimmed == "null" {
        return Ok(Vec::new());
    }

    if !trimmed.starts_with('[') {
        return Err(format!(
            "Unexpected candle response: {}",
            response_snippet(text)
        ));
    }

    let candles: Vec<Candle> = serde_json::from_str(text).map_err(|e| {
        let snippet = response_snippet(text);
        format!("Failed to parse candles: {e}\nResponse: {snippet}")
    })?;

    Ok(normalize_candles(candles))
}
