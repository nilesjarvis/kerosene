use crate::helpers::text_excerpt;
use serde::de::DeserializeOwned;
use serde_json::Value;

const HTTP_ERROR_PREVIEW_CHARS: usize = 180;

// ---------------------------------------------------------------------------
// Account Analytics HTTP Helpers
// ---------------------------------------------------------------------------

pub(super) async fn post_info_json<T>(
    client: &reqwest::Client,
    url: &str,
    label: &'static str,
    payload: Value,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("{label} request failed: {e}"))?;

    response_json(label, response).await
}

pub(super) async fn response_json<T>(
    label: &'static str,
    response: reqwest::Response,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let preview = text_excerpt(&body, HTTP_ERROR_PREVIEW_CHARS);
        return if preview.is_empty() {
            Err(format!("{label} request failed with HTTP {status}"))
        } else {
            Err(format!(
                "{label} request failed with HTTP {status}: {preview}"
            ))
        };
    }

    response
        .json::<T>()
        .await
        .map_err(|e| format!("{label} parse failed: {e}"))
}

pub(super) async fn optional_response_value(
    response: Result<reqwest::Response, reqwest::Error>,
) -> Option<Value> {
    let response = response.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<Value>().await.ok()
}

#[cfg(test)]
mod tests;
