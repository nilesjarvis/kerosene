use crate::helpers::sensitive_response_snippet;
use reqwest::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// OpenRouter REST API
// ---------------------------------------------------------------------------
//
// Foundation client for AI-assisted features (news and TradFi filing
// summaries). Components build a `ChatCompletionRequest` and call
// `chat_completion` with the key from
// `TradingTerminal::openrouter_api_key_for_task()` and the model from
// `TradingTerminal::openrouter_model_for_task()`.

const KEROSENE_USER_AGENT: &str = concat!("Kerosene/", env!("CARGO_PKG_VERSION"));
// Optional attribution header recognised by OpenRouter for app rankings.
const OPENROUTER_APP_TITLE_HEADER: &str = "X-OpenRouter-Title";
const OPENROUTER_APP_TITLE: &str = "Kerosene";

pub const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1";
/// Model used when the user has not configured one: OpenRouter's auto router.
pub const DEFAULT_OPENROUTER_MODEL: &str = "openrouter/auto";

// Completions can run far longer than the shared api::CLIENT 15s budget, so
// OpenRouter requests use a dedicated client with a generous total timeout.
static OPENROUTER_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(5))
        .pool_idle_timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| Client::new())
});

// ---------------------------------------------------------------------------
// Chat Completions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)] // constructed by upcoming AI components
pub(crate) enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ChatMessage {
    pub(crate) role: ChatRole,
    pub(crate) content: String,
}

#[allow(dead_code)] // request builders for upcoming AI components
impl ChatMessage {
    pub(crate) fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
        }
    }

    pub(crate) fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ChatCompletionRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<f32>,
}

#[allow(dead_code)] // request builders for upcoming AI components
impl ChatCompletionRequest {
    pub(crate) fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens: None,
            temperature: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChatCompletion {
    #[allow(dead_code)] // reported model can differ from the requested router slug
    pub(crate) model: String,
    pub(crate) content: String,
    pub(crate) finish_reason: Option<String>,
    #[allow(dead_code)] // usage reporting for upcoming AI components
    pub(crate) usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // usage reporting for upcoming AI components
pub(crate) struct TokenUsage {
    pub(crate) prompt_tokens: u64,
    pub(crate) completion_tokens: u64,
    pub(crate) total_tokens: u64,
}

#[derive(Deserialize)]
struct RawChatCompletionResponse {
    #[serde(default)]
    model: String,
    #[serde(default)]
    choices: Vec<RawChatChoice>,
    #[serde(default)]
    usage: Option<RawTokenUsage>,
    // Some failures arrive with HTTP 200 and a top-level error object.
    #[serde(default)]
    error: Option<RawErrorBody>,
}

#[derive(Deserialize)]
struct RawChatChoice {
    message: RawAssistantMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct RawAssistantMessage {
    #[serde(default)]
    content: Option<RawMessageContent>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawMessageContent {
    Text(String),
    Parts(Vec<RawContentPart>),
}

#[derive(Deserialize)]
struct RawContentPart {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct RawTokenUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

/// Run a non-streaming chat completion. Streaming can be layered on later
/// without changing callers that want a single summarized result.
#[allow(dead_code)] // foundation entry point for upcoming AI components
pub(crate) async fn chat_completion(
    request: ChatCompletionRequest,
    api_key: Zeroizing<String>,
) -> Result<ChatCompletion, String> {
    if api_key.trim().is_empty() {
        return Err(
            "OpenRouter API key is required; add one in Settings > Integrations".to_string(),
        );
    }
    if request.model.trim().is_empty() {
        return Err("OpenRouter chat completion request missing model".to_string());
    }
    if request.messages.is_empty() {
        return Err("OpenRouter chat completion request missing messages".to_string());
    }

    let response = OPENROUTER_CLIENT
        .clone()
        .post(format!("{OPENROUTER_API_URL}/chat/completions"))
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .header(OPENROUTER_APP_TITLE_HEADER, OPENROUTER_APP_TITLE)
        .bearer_auth(api_key.trim())
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OpenRouter chat completion request failed: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("OpenRouter chat completion response read failed: {e}"))?;
    if !status.is_success() {
        return Err(openrouter_http_error(
            "chat completion",
            status.as_u16(),
            &text,
        ));
    }
    parse_chat_completion_response(&text)
}

fn parse_chat_completion_response(text: &str) -> Result<ChatCompletion, String> {
    let raw: RawChatCompletionResponse = serde_json::from_str(text)
        .map_err(|e| format!("OpenRouter chat completion parse failed: {e}"))?;
    if let Some(error) = raw.error {
        return Err(format!(
            "OpenRouter chat completion failed: {}",
            sensitive_response_snippet(&error.message)
        ));
    }
    let Some(choice) = raw.choices.into_iter().next() else {
        return Err("OpenRouter chat completion returned no choices".to_string());
    };
    let content = match choice.message.content {
        Some(RawMessageContent::Text(text)) => text,
        Some(RawMessageContent::Parts(parts)) => parts
            .into_iter()
            .map(|part| part.text)
            .collect::<Vec<_>>()
            .join(""),
        None => String::new(),
    };
    if content.trim().is_empty() {
        return Err("OpenRouter chat completion returned no content".to_string());
    }
    Ok(ChatCompletion {
        model: raw.model,
        content,
        finish_reason: choice.finish_reason,
        usage: raw.usage.map(|usage| TokenUsage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }),
    })
}

// ---------------------------------------------------------------------------
// Key Status
// ---------------------------------------------------------------------------

/// Credit/limit state of the configured key, used to validate a key when it
/// is saved in Settings > Integrations.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct OpenRouterKeyStatus {
    pub(crate) usage_usd: f64,
    pub(crate) limit_usd: Option<f64>,
    pub(crate) limit_remaining_usd: Option<f64>,
    pub(crate) is_free_tier: bool,
}

impl std::fmt::Debug for OpenRouterKeyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpenRouterKeyStatus { <redacted> }")
    }
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

    let response = OPENROUTER_CLIENT
        .clone()
        .get(format!("{OPENROUTER_API_URL}/key"))
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|e| format!("OpenRouter key check request failed: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("OpenRouter key check response read failed: {e}"))?;
    if !status.is_success() {
        return Err(openrouter_http_error("key check", status.as_u16(), &text));
    }
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

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RawErrorEnvelope {
    error: RawErrorBody,
}

#[derive(Deserialize)]
struct RawErrorBody {
    #[serde(default)]
    message: String,
}

fn openrouter_http_error(context: &str, status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<RawErrorEnvelope>(body)
        .ok()
        .map(|envelope| envelope.error.message)
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| body.to_string());
    let hint = match status {
        401 => " (invalid or disabled API key)",
        402 => " (insufficient OpenRouter credits)",
        403 => " (request was refused or moderated)",
        408 => " (request timed out)",
        429 => " (rate limited)",
        502 | 503 => " (model or provider unavailable)",
        _ => "",
    };
    format!(
        "OpenRouter {context} HTTP {status}{hint}: {}",
        sensitive_response_snippet(&detail)
    )
}

#[cfg(test)]
mod tests;
