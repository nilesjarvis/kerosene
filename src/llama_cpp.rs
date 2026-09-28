use crate::network_activity::HttpRequestExt as _;
use reqwest::Url;
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

mod discovery;

use discovery::detection_candidates;

const MAX_DISCOVERED_MODELS: usize = 16;
const MAX_MODEL_ID_CHARS: usize = 200;

// ---------------------------------------------------------------------------
// Local llama.cpp Discovery
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LlamaCppModel {
    pub(crate) id: String,
    pub(crate) context_window: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LlamaCppServer {
    /// OpenAI-compatible API root, always normalized to a loopback `/v1` URL.
    pub(crate) base_url: String,
    pub(crate) models: Vec<LlamaCppModel>,
    pub(crate) supports_tools: bool,
    pub(crate) supports_vision: bool,
    pub(crate) supports_reasoning: bool,
}

impl LlamaCppServer {
    pub(crate) fn primary_model(&self) -> Option<&LlamaCppModel> {
        self.models.first()
    }

    pub(crate) fn endpoint_label(&self) -> String {
        Url::parse(&self.base_url)
            .ok()
            .and_then(|url| {
                let host = url.host_str()?;
                Some(match url.port() {
                    Some(port) => format!("{host}:{port}"),
                    None => host.to_string(),
                })
            })
            .unwrap_or_else(|| "local machine".to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<RawModel>,
}

#[derive(Debug, Deserialize)]
struct RawModel {
    id: String,
    #[serde(default)]
    meta: RawModelMeta,
}

#[derive(Debug, Default, Deserialize)]
struct RawModelMeta {
    n_ctx: Option<u64>,
}

pub(crate) async fn detect_server() -> Result<Option<LlamaCppServer>, String> {
    let candidates = detection_candidates()?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(250))
        .timeout(Duration::from_millis(850))
        .no_proxy()
        .build()
        .map_err(|error| format!("Could not prepare local model detection: {error}"))?;

    for base_url in candidates {
        if let Some(server) = probe_server(&client, &base_url).await {
            return Ok(Some(server));
        }
    }
    Ok(None)
}

async fn probe_server(client: &reqwest::Client, base_url: &str) -> Option<LlamaCppServer> {
    let root_url = base_url.strip_suffix("/v1")?;
    let props = client
        .get(format!("{root_url}/props"))
        .send_observed()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<Value>()
        .await
        .ok()?;
    if !looks_like_llama_cpp_props(&props) {
        return None;
    }

    let catalog = client
        .get(format!("{base_url}/models"))
        .send_observed()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<ModelsResponse>()
        .await
        .ok()?;

    let fallback_context = props
        .pointer("/default_generation_settings/n_ctx")
        .and_then(Value::as_u64);
    let models = catalog
        .data
        .into_iter()
        .take(MAX_DISCOVERED_MODELS)
        .filter_map(|model| {
            let id = bounded_model_id(&model.id)?;
            Some(LlamaCppModel {
                id,
                context_window: model.meta.n_ctx.or(fallback_context),
            })
        })
        .collect::<Vec<_>>();
    if models.is_empty() {
        return None;
    }

    let supports_tools = props
        .pointer("/chat_template_caps/supports_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && props
            .pointer("/chat_template_caps/supports_tool_calls")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    Some(LlamaCppServer {
        base_url: base_url.to_string(),
        models,
        supports_tools,
        supports_vision: props
            .pointer("/modalities/vision")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        supports_reasoning: props
            .pointer("/chat_template_caps/supports_preserve_reasoning")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn looks_like_llama_cpp_props(props: &Value) -> bool {
    props.get("chat_template_caps").is_some()
        && props.get("default_generation_settings").is_some()
        && (props.get("model_path").is_some() || props.get("build_info").is_some())
}

fn bounded_model_id(id: &str) -> Option<String> {
    let id = id.trim();
    if id.is_empty()
        || id.chars().any(char::is_control)
        || id.chars().nth(MAX_MODEL_ID_CHARS).is_some()
    {
        return None;
    }
    Some(id.to_string())
}

// ---------------------------------------------------------------------------
// Pi Provider Configuration
// ---------------------------------------------------------------------------

pub(crate) fn pi_models_config(server: &LlamaCppServer) -> Value {
    let models = server
        .models
        .iter()
        .map(|model| {
            let mut value = json!({
                "id": model.id,
                "name": format!("{} (Local)", model.id),
                "reasoning": server.supports_reasoning,
                "input": if server.supports_vision {
                    vec!["text", "image"]
                } else {
                    vec!["text"]
                },
                "cost": {
                    "input": 0,
                    "output": 0,
                    "cacheRead": 0,
                    "cacheWrite": 0
                }
            });
            if let Some(context_window) = model.context_window {
                value["contextWindow"] = json!(context_window);
                value["maxTokens"] = json!(context_window.saturating_div(4).clamp(1_024, 16_384));
            }
            value
        })
        .collect::<Vec<_>>();
    json!({
        "providers": {
            "llamacpp": {
                "name": "llama.cpp (Local)",
                "baseUrl": server.base_url,
                "api": "openai-completions",
                "apiKey": "local",
                "compat": {
                    "supportsDeveloperRole": false,
                    "supportsReasoningEffort": false
                },
                "models": models
            }
        }
    })
}

#[cfg(test)]
mod tests;
