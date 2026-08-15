use iced::{widget::markdown, window};
use std::fmt;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Kerosene Assistant State
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
    },
    Tool {
        call_id: String,
        name: String,
        finished: bool,
        is_error: bool,
    },
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
        }
    }
}

pub(crate) struct AgentState {
    pub(crate) window_id: Option<window::Id>,
    pub(crate) input: String,
    pub(crate) entries: Vec<AgentChatEntry>,
    pub(crate) status: AgentStatus,
    pub(crate) status_detail: Option<String>,
    pub(crate) runtime_connected: bool,
    pub(crate) runtime_generation: u64,
    pub(crate) snapshot_request_id: u64,
    pub(crate) pending_prompt: Option<AgentPrompt>,
    pub(crate) assistant_entry_index: Option<usize>,
    pub(crate) current_turn_has_text: bool,
    pub(crate) empty_response_retry_count: u8,
    pub(crate) suppress_empty_response_retry: bool,
    pub(crate) total_tokens: Option<u64>,
    pub(crate) total_cost_usd: Option<f64>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            window_id: None,
            input: String::new(),
            entries: Vec::new(),
            status: AgentStatus::Stopped,
            status_detail: None,
            runtime_connected: false,
            runtime_generation: 0,
            snapshot_request_id: 0,
            pending_prompt: None,
            assistant_entry_index: None,
            current_turn_has_text: false,
            empty_response_retry_count: 0,
            suppress_empty_response_retry: false,
            total_tokens: None,
            total_cost_usd: None,
        }
    }
}

impl AgentState {
    pub(crate) fn begin_new_runtime(&mut self) -> u64 {
        self.runtime_generation = self.runtime_generation.wrapping_add(1);
        self.runtime_generation
    }

    pub(crate) fn begin_snapshot(&mut self, prompt: AgentPrompt) -> (u64, u64) {
        self.snapshot_request_id = self.snapshot_request_id.wrapping_add(1);
        self.pending_prompt = Some(prompt);
        self.status = AgentStatus::Preparing;
        self.status_detail = None;
        self.current_turn_has_text = false;
        self.empty_response_retry_count = 0;
        self.suppress_empty_response_retry = false;
        (self.runtime_generation, self.snapshot_request_id)
    }

    pub(crate) fn append_assistant_delta(&mut self, delta: &str) {
        if !delta.trim().is_empty() {
            self.current_turn_has_text = true;
        }
        let entry_index = self.assistant_entry_index.unwrap_or_else(|| {
            self.entries.push(AgentChatEntry::Message {
                role: AgentChatRole::Assistant,
                text: String::new(),
                markdown: Some(Box::new(markdown::Content::new())),
            });
            let index = self.entries.len().saturating_sub(1);
            self.assistant_entry_index = Some(index);
            index
        });

        if let Some(AgentChatEntry::Message { text, markdown, .. }) =
            self.entries.get_mut(entry_index)
        {
            text.push_str(delta);
            if let Some(markdown) = markdown {
                markdown.push_str(delta);
            } else {
                *markdown = Some(Box::new(markdown::Content::parse(text)));
            }
        }
    }

    pub(crate) fn finish_tool(&mut self, call_id: &str, is_error: bool) {
        if let Some(AgentChatEntry::Tool {
            finished,
            is_error: entry_is_error,
            ..
        }) = self.entries.iter_mut().rev().find(
            |entry| matches!(entry, AgentChatEntry::Tool { call_id: id, .. } if id == call_id),
        ) {
            *finished = true;
            *entry_is_error = is_error;
        }
    }

    pub(crate) fn reset_session(&mut self) {
        self.input.clear();
        self.entries.clear();
        self.status = AgentStatus::Stopped;
        self.status_detail = None;
        self.runtime_connected = false;
        self.pending_prompt = None;
        self.assistant_entry_index = None;
        self.current_turn_has_text = false;
        self.empty_response_retry_count = 0;
        self.suppress_empty_response_retry = false;
        self.total_tokens = None;
        self.total_cost_usd = None;
        self.begin_new_runtime();
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct AgentPrompt(Zeroizing<String>);

impl AgentPrompt {
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn into_string(self) -> String {
        self.0.to_string()
    }
}

impl From<String> for AgentPrompt {
    fn from(value: String) -> Self {
        Self(value.into())
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
mod tests {
    use super::*;

    #[test]
    fn prompt_debug_is_redacted() {
        let prompt = AgentPrompt::from("private trading thesis".to_string());
        assert_eq!(format!("{prompt:?}"), "AgentPrompt(<redacted>)");
    }

    #[test]
    fn streaming_deltas_share_one_assistant_entry() {
        let mut state = AgentState::default();
        state.append_assistant_delta("Hello");
        state.append_assistant_delta(" world");

        assert!(matches!(
            state.entries.as_slice(),
            [AgentChatEntry::Message {
                role: AgentChatRole::Assistant,
                text,
                ..
            }] if text == "Hello world"
        ));
    }

    #[test]
    fn chat_entry_debug_redacts_message_text() {
        let entry = AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "private portfolio question".to_string(),
            markdown: None,
        };
        let debug = format!("{entry:?}");
        assert!(!debug.contains("private portfolio question"));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn streamed_assistant_markdown_builds_rich_blocks_incrementally() {
        let mut state = AgentState::default();
        state.append_assistant_delta("## Risk summary\n\n- **BTC** exposure\n\n");
        state.append_assistant_delta("```rust\nlet risk = 42;\n```\n");

        let [
            AgentChatEntry::Message {
                markdown: Some(markdown),
                ..
            },
        ] = state.entries.as_slice()
        else {
            panic!("expected one parsed assistant message");
        };

        assert!(matches!(
            markdown.items(),
            [
                markdown::Item::Heading(..),
                markdown::Item::List { .. },
                markdown::Item::CodeBlock { .. }
            ]
        ));
    }

    #[test]
    fn agent_uri_debug_is_redacted() {
        let uri = AgentUri::from("https://example.com/private?token=secret".to_string());
        let debug = format!("{uri:?}");

        assert_eq!(debug, "AgentUri(<redacted>)");
        assert!(!debug.contains("token=secret"));
    }
}
