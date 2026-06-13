use super::{Candle, normalize_candles};
use crate::helpers::{response_snippet, sensitive_response_snippet};
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
    redact_sensitive: bool,
) -> Result<Vec<Candle>, String> {
    let trimmed = text.trim();
    if !status.is_success() {
        return Err(format!(
            "Candle request failed (HTTP {}): {}",
            status,
            candle_response_snippet(text, redact_sensitive)
        ));
    }

    if content_type.is_some_and(|value| !value.to_ascii_lowercase().contains("json"))
        && !trimmed.starts_with('[')
        && trimmed != "null"
    {
        return Err(format!(
            "Candle request returned {} instead of JSON: {}",
            content_type.unwrap_or("unknown content"),
            candle_response_snippet(text, redact_sensitive)
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
            candle_response_snippet(text, redact_sensitive)
        ));
    }

    let candles: Vec<Candle> = serde_json::from_str(text).map_err(|e| {
        let snippet = candle_response_snippet(text, redact_sensitive);
        format!("Failed to parse candles: {e}\nResponse: {snippet}")
    })?;

    Ok(normalize_candles(candles))
}

fn candle_response_snippet(text: &str, redact_sensitive: bool) -> String {
    if redact_sensitive {
        sensitive_response_snippet(text)
    } else {
        response_snippet(text)
    }
}
