use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Ollama Client
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct OllamaChatMessage {
    pub(super) role: String,
    pub(super) content: String,
}

#[derive(Debug, Clone, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelTag>,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaModelTag {
    name: String,
}

pub async fn list_models(ollama_url: &str) -> Result<Vec<String>, String> {
    let client = crate::api::CLIENT.clone();
    let base = ollama_url.trim_end_matches('/');
    let url = format!("{base}/api/tags");
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to query Ollama models: {e}"))?;
    let parsed: OllamaTagsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama model list: {e}"))?;
    let mut models: Vec<String> = parsed.models.into_iter().map(|model| model.name).collect();
    models.sort();
    Ok(models)
}

pub(super) async fn chat_once(
    ollama_url: &str,
    model: &str,
    messages: Vec<OllamaChatMessage>,
) -> Result<String, String> {
    let client = crate::api::CLIENT.clone();
    let base = ollama_url.trim_end_matches('/');
    let url = format!("{base}/api/chat");
    let request = OllamaChatRequest {
        model: model.to_string(),
        messages,
        stream: false,
    };
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Ollama chat request failed: {e}"))?;
    let parsed: OllamaChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Ollama chat parse failed: {e}"))?;
    Ok(parsed.message.content)
}
