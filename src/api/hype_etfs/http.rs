use super::super::CLIENT;
use crate::helpers::response_excerpt;

use flate2::read::GzDecoder;
use serde::Deserialize;
use std::io::Read;

// ---------------------------------------------------------------------------
// HYPE ETF HTTP Helpers
// ---------------------------------------------------------------------------

pub(super) async fn fetch_json<T>(url: &str, label: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = CLIENT
        .clone()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{label} request failed: {e}"))?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("{label} response read failed: {e}"))?;
    let text = decode_response_text(&bytes, label)?;

    if !status.is_success() {
        return Err(format!(
            "{label} request failed (HTTP {}): {}",
            status,
            response_excerpt(&text)
        ));
    }

    serde_json::from_str(&text).map_err(|e| {
        format!(
            "{label} response parse failed: {e}; {}",
            response_excerpt(&text)
        )
    })
}

fn decode_response_text(bytes: &[u8], label: &str) -> Result<String, String> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(bytes);
        let mut text = String::new();
        decoder
            .read_to_string(&mut text)
            .map_err(|e| format!("{label} gzip response decode failed: {e}"))?;
        return Ok(text);
    }

    String::from_utf8(bytes.to_vec()).map_err(|e| format!("{label} response was not UTF-8: {e}"))
}
