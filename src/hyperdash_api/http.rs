use crate::network_activity::HttpRequestExt as _;
use reqwest::RequestBuilder;

use super::errors::hyperdash_http_error;

// ---------------------------------------------------------------------------
// HyperDash Text Response Transport
// ---------------------------------------------------------------------------

pub(super) async fn request_text(
    request: RequestBuilder,
    context: &str,
    scope: &str,
) -> Result<String, String> {
    let response = request
        .send_observed()
        .await
        .map_err(|e| format!("{context} request failed: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read {context} response: {e}"))?;

    if !status.is_success() {
        return Err(hyperdash_http_error(scope, status, &text));
    }

    Ok(text)
}

#[cfg(test)]
mod tests;
