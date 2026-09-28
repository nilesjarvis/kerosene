use super::{
    DEFAULT_OPENROUTER_MODEL, KEROSENE_USER_AGENT, OPENROUTER_API_URL, OPENROUTER_APP_TITLE,
    OPENROUTER_APP_TITLE_HEADER, OPENROUTER_CLIENT, send_request,
};
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::collections::HashSet;
use zeroize::Zeroizing;

const MAX_OPENROUTER_MODELS: usize = 1_000;
const MAX_OPENROUTER_MODEL_ID_CHARS: usize = 200;
const MAX_OPENROUTER_MODEL_NAME_CHARS: usize = 160;

// ---------------------------------------------------------------------------
// Tool-capable model catalog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OpenRouterModel {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) context_length: Option<u64>,
    pub(crate) prompt_price_per_million_usd: Option<f64>,
    pub(crate) completion_price_per_million_usd: Option<f64>,
    pub(crate) reasoning_price_per_million_usd: Option<f64>,
    pub(crate) request_price_usd: Option<f64>,
    pub(crate) has_conditional_pricing: bool,
    pub(crate) supports_image_input: bool,
}

impl OpenRouterModel {
    fn auto_router() -> Self {
        Self {
            id: DEFAULT_OPENROUTER_MODEL.to_string(),
            name: "OpenRouter Auto".to_string(),
            context_length: None,
            prompt_price_per_million_usd: None,
            completion_price_per_million_usd: None,
            reasoning_price_per_million_usd: None,
            request_price_usd: None,
            has_conditional_pricing: true,
            supports_image_input: true,
        }
    }

    pub(crate) fn pricing_summary(&self) -> String {
        let mut summary = match (
            self.prompt_price_per_million_usd,
            self.completion_price_per_million_usd,
        ) {
            (Some(prompt), Some(completion)) if prompt == 0.0 && completion == 0.0 => {
                "Free input/output".to_string()
            }
            (Some(prompt), Some(completion)) => format!(
                "{}/M input · {}/M output",
                format_usd_price(prompt),
                format_usd_price(completion)
            ),
            (Some(prompt), None) => format!(
                "{}/M input · output price unavailable",
                format_usd_price(prompt)
            ),
            (None, Some(completion)) => format!(
                "input price unavailable · {}/M output",
                format_usd_price(completion)
            ),
            (None, None) if self.id == DEFAULT_OPENROUTER_MODEL => {
                "Pricing varies by routed model".to_string()
            }
            (None, None) => "Pricing unavailable".to_string(),
        };
        if let Some(reasoning) = self
            .reasoning_price_per_million_usd
            .filter(|price| *price > 0.0)
        {
            summary.push_str(&format!(" · {}/M reasoning", format_usd_price(reasoning)));
        }
        if let Some(request) = self.request_price_usd.filter(|price| *price > 0.0) {
            summary.push_str(&format!(" · {}/request", format_usd_price(request)));
        }
        if self.has_conditional_pricing && self.id != DEFAULT_OPENROUTER_MODEL {
            summary.push_str(" · variable rates");
        }
        summary
    }

    pub(crate) fn context_summary(&self) -> String {
        self.context_length
            .map(|tokens| format!("{} context", compact_model_tokens(tokens)))
            .unwrap_or_else(|| "Dynamic context".to_string())
    }

    pub(crate) fn provider_summary(&self) -> &str {
        let provider = self.id.split('/').next().unwrap_or_default();
        match provider {
            "openai" => "OpenAI",
            "anthropic" => "Anthropic",
            "google" => "Google",
            "meta-llama" => "Meta",
            "mistralai" => "Mistral AI",
            "x-ai" => "xAI",
            "deepseek" => "DeepSeek",
            "openrouter" => "OpenRouter",
            "" => "Unknown provider",
            value => value,
        }
    }
}

#[derive(Deserialize)]
struct RawModelCatalogEnvelope {
    data: Vec<RawOpenRouterModel>,
}

#[derive(Deserialize)]
struct RawOpenRouterModel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    context_length: Option<u64>,
    #[serde(default)]
    pricing: Option<RawModelPricing>,
    #[serde(default)]
    supported_parameters: Vec<String>,
    #[serde(default)]
    architecture: Option<RawModelArchitecture>,
}

#[derive(Default, Deserialize)]
struct RawModelArchitecture {
    #[serde(default)]
    input_modalities: Vec<String>,
}

#[derive(Default, Deserialize)]
struct RawModelPricing {
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    completion: Option<String>,
    #[serde(default)]
    internal_reasoning: Option<String>,
    #[serde(default)]
    request: Option<String>,
    #[serde(default)]
    overrides: Vec<serde_json::Value>,
}

pub(crate) async fn fetch_tool_models(
    api_key: Zeroizing<String>,
) -> Result<Vec<OpenRouterModel>, String> {
    if api_key.trim().is_empty() {
        return Err("OpenRouter API key is required to load models".to_string());
    }

    let text = send_request(
        OPENROUTER_CLIENT
            .get(format!("{OPENROUTER_API_URL}/models"))
            .header(USER_AGENT, KEROSENE_USER_AGENT)
            .header(OPENROUTER_APP_TITLE_HEADER, OPENROUTER_APP_TITLE)
            .bearer_auth(api_key.trim())
            .query(&[
                ("supported_parameters", "tools"),
                ("output_modalities", "text"),
                ("sort", "most-popular"),
                ("limit", "1000"),
            ])
            .timeout(std::time::Duration::from_secs(20)),
        "model catalog",
    )
    .await?;
    parse_model_catalog_response(&text)
}

fn parse_model_catalog_response(text: &str) -> Result<Vec<OpenRouterModel>, String> {
    let raw: RawModelCatalogEnvelope = serde_json::from_str(text)
        .map_err(|e| format!("OpenRouter model catalog parse failed: {e}"))?;
    let mut seen = HashSet::new();
    let mut models = Vec::with_capacity(raw.data.len().min(MAX_OPENROUTER_MODELS) + 1);

    for raw_model in raw.data.into_iter().take(MAX_OPENROUTER_MODELS) {
        let id = raw_model.id.trim();
        if id.is_empty()
            || id.chars().count() > MAX_OPENROUTER_MODEL_ID_CHARS
            || id == DEFAULT_OPENROUTER_MODEL
            || !raw_model
                .supported_parameters
                .iter()
                .any(|parameter| parameter == "tools")
            || !seen.insert(id.to_string())
        {
            continue;
        }

        let pricing = raw_model.pricing.unwrap_or_default();
        let supports_image_input = raw_model
            .architecture
            .unwrap_or_default()
            .input_modalities
            .iter()
            .any(|modality| modality == "image");
        let name = bounded_model_name(raw_model.name.trim(), id);
        models.push(OpenRouterModel {
            id: id.to_string(),
            name,
            context_length: raw_model.context_length.filter(|length| *length > 0),
            prompt_price_per_million_usd: price_per_million(pricing.prompt.as_deref()),
            completion_price_per_million_usd: price_per_million(pricing.completion.as_deref()),
            reasoning_price_per_million_usd: price_per_million(
                pricing.internal_reasoning.as_deref(),
            ),
            request_price_usd: parsed_non_negative_price(pricing.request.as_deref()),
            has_conditional_pricing: !pricing.overrides.is_empty(),
            supports_image_input,
        });
    }

    models.insert(0, OpenRouterModel::auto_router());
    Ok(models)
}

fn price_per_million(value: Option<&str>) -> Option<f64> {
    parsed_non_negative_price(value).and_then(|price| {
        let per_million = price * 1_000_000.0;
        per_million.is_finite().then_some(per_million)
    })
}

fn parsed_non_negative_price(value: Option<&str>) -> Option<f64> {
    value?
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|price| price.is_finite() && *price >= 0.0)
}

fn bounded_model_name(name: &str, fallback: &str) -> String {
    let name = if name.is_empty() { fallback } else { name };
    let mut chars = name.chars();
    let mut bounded = chars
        .by_ref()
        .take(MAX_OPENROUTER_MODEL_NAME_CHARS)
        .collect::<String>();
    if chars.next().is_some() {
        let _ = bounded.pop();
        bounded.push('…');
    }
    bounded
}

fn format_usd_price(price: f64) -> String {
    let precision = if price >= 100.0 {
        0
    } else if price >= 1.0 {
        2
    } else if price >= 0.01 {
        3
    } else {
        4
    };
    format!("${price:.precision$}")
}

fn compact_model_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.0}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests;
