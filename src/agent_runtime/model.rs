use crate::config::AssistantProvider;
use crate::llama_cpp::LlamaCppServer;
use std::fmt;
use std::path::PathBuf;
use zeroize::Zeroizing;

pub(crate) struct AgentRuntimeConfig {
    pub(crate) generation: u64,
    pub(crate) provider: AssistantProvider,
    pub(crate) model: String,
    pub(crate) api_key: Zeroizing<String>,
    pub(crate) hyperdash_api_key: Zeroizing<String>,
    pub(crate) workspace_dir: PathBuf,
    pub(crate) local_server: Option<LlamaCppServer>,
}

#[derive(Clone)]
pub(crate) enum AgentRuntimeEvent {
    Ready {
        generation: u64,
    },
    Thinking {
        generation: u64,
    },
    ReasoningStarted {
        generation: u64,
    },
    ReasoningDelta {
        generation: u64,
        delta: String,
    },
    ReasoningFinished {
        generation: u64,
    },
    TextDelta {
        generation: u64,
        delta: String,
        total_tokens: Option<u64>,
        total_cost_usd: Option<f64>,
    },
    ToolStarted {
        generation: u64,
        call_id: String,
        name: String,
        detail: Option<String>,
    },
    ToolFinished {
        generation: u64,
        call_id: String,
        is_error: bool,
    },
    ExtensionUiRequest {
        generation: u64,
        request_id: String,
        method: String,
        title: Option<String>,
        payload: Option<String>,
    },
    ModelContext {
        generation: u64,
        model: Option<String>,
        context_window: Option<u64>,
    },
    ContextUsage {
        generation: u64,
        context_tokens: Option<u64>,
        context_window: Option<u64>,
    },
    Settled {
        generation: u64,
        total_tokens: Option<u64>,
        total_cost_usd: Option<f64>,
        has_visible_text: Option<bool>,
    },
    Error {
        generation: u64,
        message: String,
    },
    Exited {
        generation: u64,
    },
}

impl AgentRuntimeEvent {
    pub(crate) fn generation(&self) -> u64 {
        match self {
            Self::Ready { generation }
            | Self::Thinking { generation }
            | Self::ReasoningStarted { generation }
            | Self::ReasoningDelta { generation, .. }
            | Self::ReasoningFinished { generation }
            | Self::TextDelta { generation, .. }
            | Self::ToolStarted { generation, .. }
            | Self::ToolFinished { generation, .. }
            | Self::ExtensionUiRequest { generation, .. }
            | Self::ModelContext { generation, .. }
            | Self::ContextUsage { generation, .. }
            | Self::Settled { generation, .. }
            | Self::Error { generation, .. }
            | Self::Exited { generation } => *generation,
        }
    }
}

impl fmt::Debug for AgentRuntimeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready { generation } => f.debug_tuple("Ready").field(generation).finish(),
            Self::Thinking { generation } => f.debug_tuple("Thinking").field(generation).finish(),
            Self::ReasoningStarted { generation } => {
                f.debug_tuple("ReasoningStarted").field(generation).finish()
            }
            Self::ReasoningDelta { generation, .. } => f
                .debug_struct("ReasoningDelta")
                .field("generation", generation)
                .field("delta", &"<redacted>")
                .finish(),
            Self::ReasoningFinished { generation } => f
                .debug_tuple("ReasoningFinished")
                .field(generation)
                .finish(),
            Self::TextDelta { generation, .. } => f
                .debug_struct("TextDelta")
                .field("generation", generation)
                .field("delta", &"<redacted>")
                .finish(),
            Self::ToolStarted {
                generation, name, ..
            } => f
                .debug_struct("ToolStarted")
                .field("generation", generation)
                .field("name", name)
                .finish(),
            Self::ToolFinished {
                generation,
                is_error,
                ..
            } => f
                .debug_struct("ToolFinished")
                .field("generation", generation)
                .field("is_error", is_error)
                .finish(),
            Self::ExtensionUiRequest {
                generation, method, ..
            } => f
                .debug_struct("ExtensionUiRequest")
                .field("generation", generation)
                .field("method", method)
                .field("payload", &"<redacted>")
                .finish(),
            Self::ModelContext {
                generation,
                model,
                context_window,
            } => f
                .debug_struct("ModelContext")
                .field("generation", generation)
                .field("model", &model.as_ref().map(|_| "<redacted>"))
                .field("context_window", context_window)
                .finish(),
            Self::ContextUsage {
                generation,
                context_tokens,
                context_window,
            } => f
                .debug_struct("ContextUsage")
                .field("generation", generation)
                .field("context_tokens", context_tokens)
                .field("context_window", context_window)
                .finish(),
            Self::Settled { generation, .. } => f.debug_tuple("Settled").field(generation).finish(),
            Self::Error { generation, .. } => f
                .debug_struct("Error")
                .field("generation", generation)
                .field("message", &"<redacted>")
                .finish(),
            Self::Exited { generation } => f.debug_tuple("Exited").field(generation).finish(),
        }
    }
}

#[cfg(test)]
mod tests;
