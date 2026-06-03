use crate::api::API_URL;
use crate::helpers::text_excerpt;
use serde::de::DeserializeOwned;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Account HTTP Helpers
// ---------------------------------------------------------------------------

pub(super) async fn post_info_json_with_retries(
    client: reqwest::Client,
    label: &'static str,
    payload: Value,
) -> Result<Value, String> {
    let mut last_error = String::new();

    for delay_ms in [0_u64, 500, 1_500] {
        if delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }

        let response = match client.post(API_URL).json(&payload).send().await {
            Ok(response) => response,
            Err(e) => {
                last_error = format!("{label} request failed: {e}");
                continue;
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let body_preview = text_excerpt(&body, 160);
            last_error = if body_preview.is_empty() {
                format!("{label} request failed with HTTP {status}")
            } else {
                format!("{label} request failed with HTTP {status}: {body_preview}")
            };
            if status.as_u16() == 429 {
                return Err(last_error);
            }
            continue;
        }

        match response.json::<Value>().await {
            Ok(raw) => return Ok(raw),
            Err(e) => {
                last_error = format!("{label} parse failed: {e}");
            }
        }
    }

    Err(last_error)
}

pub(super) async fn best_effort_response_vec<T>(
    label: &'static str,
    response: Result<reqwest::Response, reqwest::Error>,
    warnings: &mut Vec<String>,
) -> Vec<T>
where
    T: DeserializeOwned,
{
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            warnings.push(format!("{label} request failed: {e}"));
            return Vec::new();
        }
    };

    let status = response.status();
    if !status.is_success() {
        warnings.push(format!("{label} request failed with HTTP {status}"));
        return Vec::new();
    }

    match response.json::<Vec<T>>().await {
        Ok(items) => items,
        Err(e) => {
            warnings.push(format!("{label} parse failed: {e}"));
            Vec::new()
        }
    }
}
