use super::{KEROSENE_USER_AGENT, OPENROUTER_API_URL, OPENROUTER_CLIENT, send_request};
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Key Status
// ---------------------------------------------------------------------------

/// Credit/limit state of the configured key, used to validate a key when it
/// is saved in Settings > Integrations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OpenRouterKeyStatus {
    pub(crate) usage_usd: f64,
    pub(crate) limit_usd: Option<f64>,
    pub(crate) limit_remaining_usd: Option<f64>,
    pub(crate) is_free_tier: bool,
}

#[derive(Deserialize)]
struct RawKeyStatusEnvelope {
    data: RawKeyStatus,
}

#[derive(Deserialize)]
struct RawKeyStatus {
    #[serde(default)]
    usage: f64,
    #[serde(default)]
    limit: Option<f64>,
    #[serde(default)]
    limit_remaining: Option<f64>,
    #[serde(default)]
    is_free_tier: bool,
}

pub(crate) async fn fetch_key_status(
    api_key: Zeroizing<String>,
) -> Result<OpenRouterKeyStatus, String> {
    if api_key.trim().is_empty() {
        return Err("OpenRouter API key is required".to_string());
    }

    let text = send_request(
        OPENROUTER_CLIENT
            .get(format!("{OPENROUTER_API_URL}/key"))
            .header(USER_AGENT, KEROSENE_USER_AGENT)
            .bearer_auth(api_key.trim()),
        "key check",
    )
    .await?;
    parse_key_status_response(&text)
}

fn parse_key_status_response(text: &str) -> Result<OpenRouterKeyStatus, String> {
    let raw: RawKeyStatusEnvelope = serde_json::from_str(text)
        .map_err(|e| format!("OpenRouter key check parse failed: {e}"))?;
    Ok(OpenRouterKeyStatus {
        usage_usd: raw.data.usage,
        limit_usd: raw.data.limit,
        limit_remaining_usd: raw.data.limit_remaining,
        is_free_tier: raw.data.is_free_tier,
    })
}

#[cfg(test)]
mod tests;
