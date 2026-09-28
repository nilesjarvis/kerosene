use crate::agent_pnl_card::AgentPromptImage;
use iced::widget::markdown;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Assistant chat, session wire types, and redacted inputs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum AgentStatus {
    #[default]
    Stopped,
    Preparing,
    Starting,
    Thinking,
    Ready,
    Error,
}

impl AgentStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Stopped => "Offline",
            Self::Preparing => "Preparing data",
            Self::Starting => "Starting Pi",
            Self::Thinking => "Thinking",
            Self::Ready => "Ready",
            Self::Error => "Needs attention",
        }
    }

    pub(crate) fn is_busy(self) -> bool {
        matches!(self, Self::Preparing | Self::Starting | Self::Thinking)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentChatRole {
    User,
    Assistant,
}

pub(crate) enum AgentChatEntry {
    Message {
        role: AgentChatRole,
        text: String,
        markdown: Option<Box<markdown::Content>>,
        follow_ups: Vec<String>,
    },
    Tool {
        call_id: String,
        name: String,
        detail: Option<String>,
        finished: bool,
        is_error: bool,
        expanded: bool,
    },
    Reasoning {
        text: String,
        elapsed_ticks: u64,
        finished: bool,
        expanded: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgentToolPresentation {
    pub(crate) category: &'static str,
    pub(crate) title: &'static str,
    pub(crate) running_label: &'static str,
}

impl fmt::Debug for AgentChatEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message { role, .. } => f
                .debug_struct("Message")
                .field("role", role)
                .field("text", &"<redacted>")
                .finish(),
            Self::Tool {
                name,
                finished,
                is_error,
                ..
            } => f
                .debug_struct("Tool")
                .field("name", name)
                .field("finished", finished)
                .field("is_error", is_error)
                .finish(),
            Self::Reasoning {
                elapsed_ticks,
                finished,
                expanded,
                ..
            } => f
                .debug_struct("Reasoning")
                .field("text", &"<redacted>")
                .field("elapsed_ticks", elapsed_ticks)
                .field("finished", finished)
                .field("expanded", expanded)
                .finish(),
        }
    }
}

pub(crate) struct AgentStoredSession {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
    pub(crate) input: String,
    pub(crate) entries: Vec<AgentChatEntry>,
    pub(crate) requested_model: Option<String>,
    pub(crate) runtime_model: Option<String>,
    pub(crate) context_tokens: Option<u64>,
    pub(crate) context_window: Option<u64>,
    pub(crate) total_tokens: Option<u64>,
    pub(crate) total_cost_usd: Option<f64>,
}

impl fmt::Debug for AgentStoredSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentStoredSession")
            .field("id", &self.id)
            .field("title", &"<redacted>")
            .field("created_at_ms", &self.created_at_ms)
            .field("updated_at_ms", &self.updated_at_ms)
            .field("input", &"<redacted>")
            .field("entries", &format_args!("len={}", self.entries.len()))
            .field(
                "requested_model",
                &self.requested_model.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "runtime_model",
                &self.runtime_model.as_ref().map(|_| "<redacted>"),
            )
            .field("context_tokens", &self.context_tokens)
            .field("context_window", &self.context_window)
            .field("total_tokens", &self.total_tokens)
            .field("total_cost_usd", &self.total_cost_usd)
            .finish()
    }
}

pub(crate) struct AgentSessionListItem<'a> {
    pub(crate) id: u64,
    pub(crate) title: &'a str,
    pub(crate) message_count: usize,
    pub(crate) updated_at_ms: u64,
    pub(crate) active: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PersistedAgentStore {
    pub(crate) schema_version: u32,
    pub(crate) active_session_id: u64,
    pub(crate) next_session_id: u64,
    pub(crate) sessions: Vec<PersistedAgentSession>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PersistedAgentSession {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
    #[serde(default)]
    pub(crate) input: String,
    #[serde(default)]
    pub(crate) entries: Vec<PersistedAgentEntry>,
    #[serde(default)]
    pub(crate) total_tokens: Option<u64>,
    #[serde(default)]
    pub(crate) total_cost_usd: Option<f64>,
    #[serde(default)]
    pub(crate) requested_model: Option<String>,
    #[serde(default)]
    pub(crate) runtime_model: Option<String>,
    #[serde(default)]
    pub(crate) context_tokens: Option<u64>,
    #[serde(default)]
    pub(crate) context_window: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PersistedAgentEntry {
    pub(crate) role: PersistedAgentRole,
    pub(crate) text: String,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PersistedAgentRole {
    User,
    Assistant,
}

#[derive(Clone)]
pub(crate) struct AgentPersistenceResult(Result<(), String>);

impl AgentPersistenceResult {
    pub(crate) fn into_result(self) -> Result<(), String> {
        self.0
    }
}

impl From<Result<(), String>> for AgentPersistenceResult {
    fn from(value: Result<(), String>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for AgentPersistenceResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Ok(()) => f.write_str("AgentPersistenceResult(Ok)"),
            Err(_) => f.write_str("AgentPersistenceResult(Err(<redacted>))"),
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct AgentPrompt {
    text: Zeroizing<String>,
    images: Vec<AgentPromptImage>,
}

impl AgentPrompt {
    pub(crate) fn as_str(&self) -> &str {
        self.text.as_str()
    }

    pub(crate) fn into_string(self) -> String {
        self.text.to_string()
    }

    pub(crate) fn with_image(mut self, image: AgentPromptImage) -> Self {
        self.images.push(image);
        self
    }

    pub(crate) fn images(&self) -> &[AgentPromptImage] {
        &self.images
    }
}

impl From<String> for AgentPrompt {
    fn from(value: String) -> Self {
        Self {
            text: value.into(),
            images: Vec::new(),
        }
    }
}

impl fmt::Debug for AgentPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgentPrompt(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct AgentUri(Zeroizing<String>);

impl AgentUri {
    pub(crate) fn into_string(self) -> String {
        self.0.to_string()
    }
}

impl From<String> for AgentUri {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl fmt::Debug for AgentUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgentUri(<redacted>)")
    }
}

#[cfg(test)]
mod tests;
