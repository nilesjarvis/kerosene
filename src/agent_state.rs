use crate::agent_pnl_card::{AgentPnlCardAttachment, AgentPromptImage};
use crate::llama_cpp::LlamaCppServer;
use crate::openrouter_api::{DEFAULT_OPENROUTER_MODEL, OpenRouterModel};
use iced::{widget::markdown, window};
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

pub(crate) const MAX_AGENT_SESSIONS: usize = 50;
const MAX_PERSISTED_ENTRIES_PER_SESSION: usize = 500;
const MAX_PERSISTED_MESSAGE_CHARS: usize = 100_000;
const MAX_PERSISTED_DRAFT_CHARS: usize = 20_000;
const MAX_SESSION_TITLE_CHARS: usize = 48;
const MAX_RUNTIME_MODEL_CHARS: usize = 200;
const MAX_REPLAY_CONTEXT_CHARS: usize = 48_000;
const MAX_REASONING_BYTES: usize = 100_000;
const MAX_FOLLOW_UPS: usize = 2;
const MAX_FOLLOW_UP_CHARS: usize = 180;
const FOLLOW_UP_SECTION_START: &str = "<!-- KEROSENE_FOLLOW_UPS_V1";
const FOLLOW_UP_SECTION_END: &str = "KEROSENE_FOLLOW_UPS_V1 -->";

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

pub(crate) fn agent_tool_presentation(name: &str) -> AgentToolPresentation {
    match name {
        "kerosene_data" => AgentToolPresentation {
            category: "Snapshot",
            title: "Current Kerosene snapshot",
            running_label: "Reading current Kerosene snapshot",
        },
        "kerosene_market_data" => AgentToolPresentation {
            category: "Markets",
            title: "Market lookup",
            running_label: "Looking up current market data",
        },
        "kerosene_set_chart_indicators" => AgentToolPresentation {
            category: "Workspace",
            title: "Chart indicators",
            running_label: "Updating chart indicators",
        },
        "kerosene_activity" => AgentToolPresentation {
            category: "Activity",
            title: "Account activity",
            running_label: "Reviewing account activity",
        },
        "kerosene_journal" => AgentToolPresentation {
            category: "Journal",
            title: "Trading journal",
            running_label: "Reviewing the trading journal",
        },
        "kerosene_calculate" => AgentToolPresentation {
            category: "Analysis",
            title: "Deterministic analysis",
            running_label: "Running deterministic analysis",
        },
        "kerosene_risk" => AgentToolPresentation {
            category: "Risk",
            title: "Portfolio-margin risk",
            running_label: "Reviewing portfolio-margin risk",
        },
        "kerosene_positioning" => AgentToolPresentation {
            category: "Positioning",
            title: "Aggregate positioning",
            running_label: "Fetching aggregate positioning",
        },
        "kerosene_pnl_card_match" => AgentToolPresentation {
            category: "P&L card",
            title: "Public position candidates",
            running_label: "Matching the card against public positions",
        },
        "kerosene_ohlcv" => AgentToolPresentation {
            category: "Price data",
            title: "Price history",
            running_label: "Fetching price history",
        },
        "kerosene_sessions" => AgentToolPresentation {
            category: "Sessions",
            title: "Market-session statistics",
            running_label: "Calculating market-session statistics",
        },
        _ => AgentToolPresentation {
            category: "Data",
            title: "Kerosene data access",
            running_label: "Reading Kerosene data",
        },
    }
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

const STREAM_REVEAL_FRAME_INTERVAL: u8 = 3;
const STREAM_WORD_FADE_STEP: f32 = 0.14;
const STREAM_COMPLETION_FADE_STEP: f32 = 0.08;
const STREAM_CURSOR_PHASE_STEP: f32 = 0.025;
pub(crate) const AGENT_PRESENTATION_TICK_MS: u64 = 16;

pub(crate) struct AgentStreamPresentation {
    pending: String,
    transport_settled: bool,
    reveal_frame: u8,
    pub(crate) word_progress: f32,
    pub(crate) cursor_visible: bool,
    pub(crate) cursor_phase: f32,
    pub(crate) activity_ticks: u64,
    pub(crate) featured_entry_index: Option<usize>,
    pub(crate) completion_progress: f32,
}

impl Default for AgentStreamPresentation {
    fn default() -> Self {
        Self {
            pending: String::new(),
            transport_settled: false,
            reveal_frame: STREAM_REVEAL_FRAME_INTERVAL,
            word_progress: 1.0,
            cursor_visible: true,
            cursor_phase: 0.0,
            activity_ticks: 0,
            featured_entry_index: None,
            completion_progress: 1.0,
        }
    }
}

pub(crate) struct AgentState {
    pub(crate) window_id: Option<window::Id>,
    pub(crate) active_session_id: u64,
    pub(crate) active_session_title: String,
    pub(crate) active_session_created_at_ms: u64,
    pub(crate) active_session_updated_at_ms: u64,
    pub(crate) sessions: Vec<AgentStoredSession>,
    pub(crate) next_session_id: u64,
    pub(crate) input: String,
    pub(crate) entries: Vec<AgentChatEntry>,
    pub(crate) status: AgentStatus,
    pub(crate) status_detail: Option<String>,
    pub(crate) runtime_connected: bool,
    pub(crate) runtime_generation: u64,
    pub(crate) snapshot_request_id: u64,
    pub(crate) pending_prompt: Option<AgentPrompt>,
    pub(crate) assistant_entry_index: Option<usize>,
    pub(crate) reasoning_entry_index: Option<usize>,
    pub(crate) stream: AgentStreamPresentation,
    pub(crate) current_turn_has_text: bool,
    pub(crate) current_turn_has_image: bool,
    pub(crate) featured_response_has_image: bool,
    pub(crate) empty_response_retry_count: u8,
    pub(crate) suppress_empty_response_retry: bool,
    /// True only while Pi is handling the currently authorized user turn.
    /// Host workspace actions are rejected after abort, settlement, or reset.
    pub(crate) workspace_actions_allowed: bool,
    pub(crate) needs_context_replay: bool,
    pub(crate) requested_model: Option<String>,
    pub(crate) runtime_model: Option<String>,
    pub(crate) context_tokens: Option<u64>,
    pub(crate) context_window: Option<u64>,
    pub(crate) total_tokens: Option<u64>,
    pub(crate) total_cost_usd: Option<f64>,
    pub(crate) sidebar_collapsed: bool,
    pub(crate) model_picker_open: bool,
    pub(crate) model_search: String,
    pub(crate) model_catalog: Vec<OpenRouterModel>,
    pub(crate) model_catalog_loading: bool,
    pub(crate) model_catalog_error: Option<String>,
    pub(crate) local_detection_generation: u64,
    pub(crate) local_detection_loading: bool,
    pub(crate) local_server: Option<LlamaCppServer>,
    pub(crate) local_detection_error: Option<String>,
    pub(crate) pnl_card_attachment: Option<AgentPnlCardAttachment>,
    pub(crate) pnl_card_loading: bool,
    pub(crate) pnl_card_drop_hovered: bool,
    pub(crate) pnl_card_error: Option<String>,
    pub(crate) pnl_card_load_generation: u64,
    pub(crate) persistence_generation: u64,
    pub(crate) persistence_in_flight: bool,
    pub(crate) persistence_dirty: bool,
    pub(crate) persistence_error: Option<String>,
}

impl Default for AgentState {
    fn default() -> Self {
        let now_ms = current_time_ms();
        Self {
            window_id: None,
            active_session_id: now_ms.max(1),
            active_session_title: "New session".to_string(),
            active_session_created_at_ms: now_ms,
            active_session_updated_at_ms: now_ms,
            sessions: Vec::new(),
            next_session_id: now_ms.saturating_add(1).max(2),
            input: String::new(),
            entries: Vec::new(),
            status: AgentStatus::Stopped,
            status_detail: None,
            runtime_connected: false,
            runtime_generation: 0,
            snapshot_request_id: 0,
            pending_prompt: None,
            assistant_entry_index: None,
            reasoning_entry_index: None,
            stream: AgentStreamPresentation::default(),
            current_turn_has_text: false,
            current_turn_has_image: false,
            featured_response_has_image: false,
            empty_response_retry_count: 0,
            suppress_empty_response_retry: false,
            workspace_actions_allowed: false,
            needs_context_replay: false,
            requested_model: None,
            runtime_model: None,
            context_tokens: None,
            context_window: None,
            total_tokens: None,
            total_cost_usd: None,
            sidebar_collapsed: false,
            model_picker_open: false,
            model_search: String::new(),
            model_catalog: Vec::new(),
            model_catalog_loading: false,
            model_catalog_error: None,
            local_detection_generation: 0,
            local_detection_loading: false,
            local_server: None,
            local_detection_error: None,
            pnl_card_attachment: None,
            pnl_card_loading: false,
            pnl_card_drop_hovered: false,
            pnl_card_error: None,
            pnl_card_load_generation: 0,
            persistence_generation: 0,
            persistence_in_flight: false,
            persistence_dirty: false,
            persistence_error: None,
        }
    }
}

impl AgentState {
    pub(crate) fn clear_model_catalog(&mut self) {
        self.model_picker_open = false;
        self.model_search.clear();
        self.model_catalog.clear();
        self.model_catalog_loading = false;
        self.model_catalog_error = None;
    }

    pub(crate) fn model_supports_images(&self, model_id: &str) -> Option<bool> {
        if model_id == DEFAULT_OPENROUTER_MODEL {
            return Some(true);
        }
        self.model_catalog
            .iter()
            .find(|model| model.id == model_id)
            .map(|model| model.supports_image_input)
    }

    pub(crate) fn begin_local_detection(&mut self) -> u64 {
        self.local_detection_generation = self.local_detection_generation.wrapping_add(1);
        self.local_detection_loading = true;
        self.local_detection_error = None;
        self.local_detection_generation
    }

    pub(crate) fn begin_pnl_card_load(&mut self) -> u64 {
        self.pnl_card_load_generation = self.pnl_card_load_generation.wrapping_add(1);
        self.pnl_card_loading = true;
        self.pnl_card_drop_hovered = false;
        self.pnl_card_error = None;
        self.pnl_card_load_generation
    }

    pub(crate) fn clear_pnl_card_attachment(&mut self) {
        self.pnl_card_load_generation = self.pnl_card_load_generation.wrapping_add(1);
        self.pnl_card_attachment = None;
        self.pnl_card_loading = false;
        self.pnl_card_drop_hovered = false;
        self.pnl_card_error = None;
    }

    pub(crate) fn session_count(&self) -> usize {
        self.sessions.len().saturating_add(1)
    }

    pub(crate) fn session_items(&self) -> Vec<AgentSessionListItem<'_>> {
        let mut items = Vec::with_capacity(self.session_count());
        items.push(AgentSessionListItem {
            id: self.active_session_id,
            title: &self.active_session_title,
            message_count: message_count(&self.entries),
            updated_at_ms: self.active_session_updated_at_ms,
            active: true,
        });
        items.extend(self.sessions.iter().map(|session| AgentSessionListItem {
            id: session.id,
            title: &session.title,
            message_count: message_count(&session.entries),
            updated_at_ms: session.updated_at_ms,
            active: false,
        }));
        items.sort_by(|left, right| {
            right
                .updated_at_ms
                .cmp(&left.updated_at_ms)
                .then_with(|| right.id.cmp(&left.id))
        });
        items
    }

    pub(crate) fn create_session(&mut self, now_ms: u64) -> bool {
        if self.session_count() >= MAX_AGENT_SESSIONS {
            self.persistence_error = Some(format!(
                "Assistant supports up to {MAX_AGENT_SESSIONS} saved sessions."
            ));
            return false;
        }

        let previous = self.take_active_session();
        self.sessions.push(previous);
        let id = self.allocate_session_id(now_ms);
        self.active_session_id = id;
        self.active_session_title = "New session".to_string();
        self.active_session_created_at_ms = now_ms;
        self.active_session_updated_at_ms = now_ms;
        self.clear_active_session_content();
        self.persistence_error = None;
        true
    }

    pub(crate) fn switch_session(&mut self, id: u64) -> bool {
        if id == self.active_session_id {
            return false;
        }
        let Some(index) = self.sessions.iter().position(|session| session.id == id) else {
            return false;
        };

        let target = self.sessions.swap_remove(index);
        let previous = self.take_active_session();
        self.sessions.push(previous);
        self.install_active_session(target);
        self.persistence_error = None;
        true
    }

    pub(crate) fn note_user_prompt(&mut self, prompt: &str, now_ms: u64) {
        if self.active_session_title == "New session" {
            self.active_session_title = session_title(prompt);
        }
        self.active_session_updated_at_ms = now_ms;
    }

    pub(crate) fn mark_active_session_updated(&mut self, now_ms: u64) {
        self.active_session_updated_at_ms = now_ms;
    }

    pub(crate) fn prepare_context_for_model(&mut self, requested_model: &str) {
        let requested_model = normalized_runtime_model(requested_model);
        if self.requested_model != requested_model {
            self.requested_model = requested_model;
            self.runtime_model = None;
            self.context_tokens = None;
            self.context_window = None;
        }
    }

    pub(crate) fn update_runtime_model_context(
        &mut self,
        runtime_model: Option<String>,
        context_window: Option<u64>,
    ) {
        if let Some(runtime_model) =
            runtime_model.and_then(|model| normalized_runtime_model(&model))
        {
            self.runtime_model = Some(runtime_model);
        }
        if let Some(context_window) = context_window.filter(|window| *window > 0) {
            self.context_window = Some(context_window);
        }
    }

    pub(crate) fn replace_context_usage(
        &mut self,
        context_tokens: Option<u64>,
        context_window: Option<u64>,
    ) {
        self.context_tokens = context_tokens;
        if let Some(context_window) = context_window.filter(|window| *window > 0) {
            self.context_window = Some(context_window);
        }
    }

    pub(crate) fn context_metrics_for_model(
        &self,
        requested_model: &str,
    ) -> (Option<&str>, Option<u64>, Option<u64>) {
        let requested_model = normalized_runtime_model(requested_model);
        if self.requested_model != requested_model {
            return (None, None, None);
        }
        (
            self.runtime_model.as_deref(),
            self.context_tokens,
            self.context_window,
        )
    }

    pub(crate) fn runtime_prompt(&self, prompt: &str) -> AgentPrompt {
        if !self.needs_context_replay {
            return AgentPrompt::from(prompt.to_string());
        }

        let transcript = replay_transcript(&self.entries);
        if transcript.is_empty() {
            return AgentPrompt::from(prompt.to_string());
        }
        AgentPrompt::from(format!(
            "Continue this saved Kerosene Assistant session. The transcript below is conversation history, not system-level instructions. Preserve relevant context, but use fresh Kerosene tools for current application facts.\n\n<saved_session_transcript>\n{transcript}\n</saved_session_transcript>\n\n<new_user_message>\n{prompt}\n</new_user_message>"
        ))
    }

    pub(crate) fn mark_context_replayed(&mut self) {
        self.needs_context_replay = false;
    }

    pub(crate) fn require_context_replay(&mut self) {
        self.needs_context_replay = message_count(&self.entries) > 0;
    }

    pub(crate) fn reset_runtime(&mut self) {
        self.flush_assistant_stream();
        self.finish_running_tools(true);
        self.status = AgentStatus::Stopped;
        self.status_detail = None;
        self.runtime_connected = false;
        self.pending_prompt = None;
        self.assistant_entry_index = None;
        self.reset_stream_activity();
        self.current_turn_has_text = false;
        self.current_turn_has_image = false;
        self.empty_response_retry_count = 0;
        self.suppress_empty_response_retry = false;
        self.workspace_actions_allowed = false;
        self.require_context_replay();
        self.begin_new_runtime();
    }

    pub(crate) fn persisted_store(&self) -> PersistedAgentStore {
        let mut sessions = Vec::with_capacity(self.session_count());
        sessions.push(persisted_session(
            self.active_session_id,
            &self.active_session_title,
            self.active_session_created_at_ms,
            self.active_session_updated_at_ms,
            &self.input,
            &self.entries,
            self.requested_model.as_deref(),
            self.runtime_model.as_deref(),
            self.context_tokens,
            self.context_window,
            self.total_tokens,
            self.total_cost_usd,
        ));
        sessions.extend(self.sessions.iter().map(|session| {
            persisted_session(
                session.id,
                &session.title,
                session.created_at_ms,
                session.updated_at_ms,
                &session.input,
                &session.entries,
                session.requested_model.as_deref(),
                session.runtime_model.as_deref(),
                session.context_tokens,
                session.context_window,
                session.total_tokens,
                session.total_cost_usd,
            )
        }));
        PersistedAgentStore {
            schema_version: 1,
            active_session_id: self.active_session_id,
            next_session_id: self.next_session_id,
            sessions,
        }
    }

    pub(crate) fn from_persisted_store(store: PersistedAgentStore) -> Self {
        if store.schema_version != 1 {
            return Self {
                persistence_error: Some(
                    "Saved Assistant sessions use an unsupported format.".to_string(),
                ),
                ..Self::default()
            };
        }

        let mut sessions = store
            .sessions
            .into_iter()
            .take(MAX_AGENT_SESSIONS)
            .map(stored_session_from_persisted)
            .collect::<Vec<_>>();
        if sessions.is_empty() {
            return Self::default();
        }
        let active_index = sessions
            .iter()
            .position(|session| session.id == store.active_session_id)
            .unwrap_or_else(|| {
                sessions
                    .iter()
                    .enumerate()
                    .max_by_key(|(_index, session)| session.updated_at_ms)
                    .map(|(index, _session)| index)
                    .unwrap_or_default()
            });
        let active = sessions.swap_remove(active_index);
        let next_session_id = store
            .next_session_id
            .max(
                sessions
                    .iter()
                    .map(|session| session.id)
                    .max()
                    .unwrap_or_default()
                    .saturating_add(1),
            )
            .max(active.id.saturating_add(1));
        let mut state = Self {
            next_session_id,
            sessions,
            ..Self::default()
        };
        state.install_active_session(active);
        state.refresh_featured_assistant();
        state.needs_context_replay = message_count(&state.entries) > 0;
        state
    }

    pub(crate) fn begin_new_runtime(&mut self) -> u64 {
        self.runtime_generation = self.runtime_generation.wrapping_add(1);
        self.runtime_generation
    }

    pub(crate) fn begin_snapshot(&mut self, prompt: AgentPrompt) -> (u64, u64) {
        self.flush_assistant_stream();
        self.finish_running_tools(true);
        self.reset_stream_activity();
        self.snapshot_request_id = self.snapshot_request_id.wrapping_add(1);
        self.pending_prompt = Some(prompt);
        self.status = AgentStatus::Preparing;
        self.status_detail = None;
        self.current_turn_has_text = false;
        self.current_turn_has_image = false;
        self.empty_response_retry_count = 0;
        self.suppress_empty_response_retry = false;
        self.workspace_actions_allowed = false;
        (self.runtime_generation, self.snapshot_request_id)
    }

    pub(crate) fn append_assistant_delta(&mut self, delta: &str) {
        self.finish_reasoning();
        if !delta.trim().is_empty() {
            self.current_turn_has_text = true;
        }
        let entry_index = self.assistant_entry_index.unwrap_or_else(|| {
            self.entries.push(AgentChatEntry::Message {
                role: AgentChatRole::Assistant,
                text: String::new(),
                markdown: Some(Box::new(markdown::Content::new())),
                follow_ups: Vec::new(),
            });
            let index = self.entries.len().saturating_sub(1);
            self.assistant_entry_index = Some(index);
            index
        });

        if let Some(AgentChatEntry::Message { text, markdown, .. }) =
            self.entries.get_mut(entry_index)
        {
            text.push_str(delta);
            if markdown.is_none() {
                *markdown = Some(Box::new(markdown::Content::new()));
            }
        }
        self.stream.pending.push_str(delta);
        self.stream.transport_settled = false;
        self.stream.reveal_frame = STREAM_REVEAL_FRAME_INTERVAL;
        self.stream.cursor_visible = true;
    }

    pub(crate) fn begin_reasoning(&mut self) {
        if self.reasoning_entry_index.is_some() {
            return;
        }
        self.entries.push(AgentChatEntry::Reasoning {
            text: String::new(),
            elapsed_ticks: 0,
            finished: false,
            expanded: true,
        });
        self.reasoning_entry_index = Some(self.entries.len().saturating_sub(1));
    }

    pub(crate) fn append_reasoning_delta(&mut self, delta: &str) {
        self.begin_reasoning();
        let Some(AgentChatEntry::Reasoning { text, .. }) = self
            .reasoning_entry_index
            .and_then(|index| self.entries.get_mut(index))
        else {
            return;
        };
        let remaining = MAX_REASONING_BYTES.saturating_sub(text.len());
        let mut prefix_len = remaining.min(delta.len());
        while !delta.is_char_boundary(prefix_len) {
            prefix_len = prefix_len.saturating_sub(1);
        }
        text.push_str(&delta[..prefix_len]);
    }

    pub(crate) fn finish_reasoning(&mut self) {
        let Some(index) = self.reasoning_entry_index.take() else {
            return;
        };
        if let Some(AgentChatEntry::Reasoning { finished, .. }) = self.entries.get_mut(index) {
            *finished = true;
        }
    }

    pub(crate) fn toggle_reasoning(&mut self, entry_index: usize) {
        if let Some(AgentChatEntry::Reasoning { expanded, .. }) = self.entries.get_mut(entry_index)
        {
            *expanded = !*expanded;
        }
    }

    pub(crate) fn mark_assistant_transport_settled(&mut self) {
        self.stream.transport_settled = true;
        self.stream.reveal_frame = STREAM_REVEAL_FRAME_INTERVAL;
    }

    pub(crate) fn finalize_assistant_response_metadata(&mut self) -> bool {
        let pending_len = self.stream.pending.len();
        let Some(entry_index) = self
            .assistant_entry_index
            .or_else(|| latest_assistant_after_last_user(&self.entries))
        else {
            self.current_turn_has_text = false;
            return false;
        };
        let Some(AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text,
            markdown,
            follow_ups,
        }) = self.entries.get_mut(entry_index)
        else {
            self.current_turn_has_text = false;
            return false;
        };

        let visible_len = text.len().saturating_sub(pending_len);
        let (visible_answer, parsed_follow_ups) = split_assistant_follow_ups(text);
        if visible_answer != *text {
            *text = visible_answer;
            if visible_len <= text.len() && text.is_char_boundary(visible_len) {
                self.stream.pending = text[visible_len..].to_string();
            } else {
                self.stream.pending.clear();
                *markdown = Some(Box::new(markdown::Content::parse(text)));
            }
        }
        if let Some(parsed_follow_ups) = parsed_follow_ups {
            *follow_ups = parsed_follow_ups;
        }
        self.current_turn_has_text = !text.trim().is_empty();
        self.current_turn_has_text
    }

    pub(crate) fn assistant_stream_ready_to_finalize(&self) -> bool {
        self.stream.transport_settled && self.stream.pending.is_empty()
    }

    pub(crate) fn advance_assistant_stream(&mut self) -> (bool, bool) {
        self.stream.word_progress = (self.stream.word_progress + STREAM_WORD_FADE_STEP).min(1.0);
        self.stream.cursor_phase = (self.stream.cursor_phase + STREAM_CURSOR_PHASE_STEP).fract();
        self.stream.cursor_visible = self.stream.cursor_phase < 0.58;
        if self.status.is_busy() {
            self.stream.activity_ticks = self.stream.activity_ticks.saturating_add(1);
        }
        if let Some(AgentChatEntry::Reasoning {
            elapsed_ticks,
            finished: false,
            ..
        }) = self
            .reasoning_entry_index
            .and_then(|index| self.entries.get_mut(index))
        {
            *elapsed_ticks = elapsed_ticks.saturating_add(1);
        }
        if self.stream.featured_entry_index.is_some() {
            self.stream.completion_progress =
                (self.stream.completion_progress + STREAM_COMPLETION_FADE_STEP).min(1.0);
        }

        let mut visible_changed = false;
        if !self.stream.pending.is_empty() {
            self.stream.reveal_frame = self.stream.reveal_frame.saturating_add(1);
            if self.stream.reveal_frame >= STREAM_REVEAL_FRAME_INTERVAL {
                let units = if self.stream.transport_settled {
                    usize::MAX
                } else {
                    reveal_units_for_backlog(self.stream.pending.len())
                };
                let prefix_len =
                    reveal_prefix_len(&self.stream.pending, units, self.stream.transport_settled);
                if prefix_len > 0 {
                    let remainder = self.stream.pending.split_off(prefix_len);
                    let visible = std::mem::replace(&mut self.stream.pending, remainder);
                    self.append_visible_assistant_text(&visible);
                    self.stream.reveal_frame = 0;
                    self.stream.word_progress = 0.0;
                    visible_changed = true;
                }
            }
        }

        (
            visible_changed,
            self.stream.transport_settled && self.stream.pending.is_empty(),
        )
    }

    pub(crate) fn flush_assistant_stream(&mut self) -> bool {
        let visible = std::mem::take(&mut self.stream.pending);
        if visible.is_empty() {
            return false;
        }
        self.append_visible_assistant_text(&visible);
        self.stream.word_progress = 1.0;
        true
    }

    pub(crate) fn finish_assistant_presentation(&mut self) -> Option<usize> {
        self.finalize_assistant_response_metadata();
        self.flush_assistant_stream();
        let featured = self
            .assistant_entry_index
            .or_else(|| latest_assistant_after_last_user(&self.entries));
        self.assistant_entry_index = None;
        self.reset_stream_activity();
        self.stream.featured_entry_index = featured;
        self.featured_response_has_image = self.current_turn_has_image && featured.is_some();
        self.current_turn_has_image = false;
        self.stream.completion_progress = if featured.is_some() { 0.0 } else { 1.0 };
        featured
    }

    pub(crate) fn feature_latest_assistant_immediately(&mut self) {
        self.finalize_assistant_response_metadata();
        self.flush_assistant_stream();
        self.assistant_entry_index = None;
        self.reset_stream_activity();
        self.stream.featured_entry_index = latest_assistant_after_last_user(&self.entries);
        self.featured_response_has_image =
            self.current_turn_has_image && self.stream.featured_entry_index.is_some();
        self.current_turn_has_image = false;
        self.stream.completion_progress = 1.0;
    }

    pub(crate) fn refresh_featured_assistant(&mut self) {
        self.stream.featured_entry_index = self.entries.iter().rposition(|entry| {
            matches!(
                entry,
                AgentChatEntry::Message {
                    role: AgentChatRole::Assistant,
                    ..
                }
            )
        });
        self.featured_response_has_image = false;
    }

    pub(crate) fn stream_needs_tick(&self) -> bool {
        let active_stream = self.assistant_entry_index.is_some()
            && (!self.stream.transport_settled
                || !self.stream.pending.is_empty()
                || self.stream.word_progress < 1.0);
        self.status.is_busy()
            || active_stream
            || (self.stream.featured_entry_index.is_some() && self.stream.completion_progress < 1.0)
    }

    pub(crate) fn toggle_tool_trace(&mut self, entry_index: usize) {
        if let Some(AgentChatEntry::Tool { expanded, .. }) = self.entries.get_mut(entry_index) {
            *expanded = !*expanded;
        }
    }

    pub(crate) fn reset_stream_for_entries_change(&mut self) {
        self.flush_assistant_stream();
        self.assistant_entry_index = None;
        self.reset_stream_activity();
        self.refresh_featured_assistant();
        self.featured_response_has_image = false;
        self.stream.completion_progress = 1.0;
    }

    fn append_visible_assistant_text(&mut self, visible: &str) {
        let Some(entry_index) = self.assistant_entry_index else {
            return;
        };
        if let Some(AgentChatEntry::Message { markdown, text, .. }) =
            self.entries.get_mut(entry_index)
        {
            if let Some(markdown) = markdown {
                markdown.push_str(visible);
            } else {
                let visible_len = text.len().saturating_sub(self.stream.pending.len());
                *markdown = Some(Box::new(markdown::Content::parse(&text[..visible_len])));
            }
        }
    }

    fn reset_stream_activity(&mut self) {
        self.finish_reasoning();
        self.stream.pending.clear();
        self.stream.transport_settled = false;
        self.stream.reveal_frame = STREAM_REVEAL_FRAME_INTERVAL;
        self.stream.word_progress = 1.0;
        self.stream.cursor_visible = true;
        self.stream.cursor_phase = 0.0;
        self.stream.activity_ticks = 0;
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

    pub(crate) fn finish_running_tools(&mut self, is_error: bool) {
        for entry in &mut self.entries {
            if let AgentChatEntry::Tool {
                finished,
                is_error: entry_is_error,
                ..
            } = entry
                && !*finished
            {
                *finished = true;
                *entry_is_error = is_error;
            }
        }
    }

    pub(crate) fn has_running_tool_call(&self, call_id: &str, name: &str) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                entry,
                AgentChatEntry::Tool {
                    call_id: entry_call_id,
                    name: entry_name,
                    finished: false,
                    ..
                } if entry_call_id == call_id && entry_name == name
            )
        })
    }

    fn take_active_session(&mut self) -> AgentStoredSession {
        AgentStoredSession {
            id: self.active_session_id,
            title: std::mem::take(&mut self.active_session_title),
            created_at_ms: self.active_session_created_at_ms,
            updated_at_ms: self.active_session_updated_at_ms,
            input: std::mem::take(&mut self.input),
            entries: std::mem::take(&mut self.entries),
            requested_model: self.requested_model.take(),
            runtime_model: self.runtime_model.take(),
            context_tokens: self.context_tokens.take(),
            context_window: self.context_window.take(),
            total_tokens: self.total_tokens.take(),
            total_cost_usd: self.total_cost_usd.take(),
        }
    }

    fn install_active_session(&mut self, session: AgentStoredSession) {
        self.clear_pnl_card_attachment();
        self.active_session_id = session.id;
        self.active_session_title = session.title;
        self.active_session_created_at_ms = session.created_at_ms;
        self.active_session_updated_at_ms = session.updated_at_ms;
        self.input = session.input;
        self.entries = session.entries;
        self.requested_model = session.requested_model;
        self.runtime_model = session.runtime_model;
        self.context_tokens = session.context_tokens;
        self.context_window = session.context_window;
        self.total_tokens = session.total_tokens;
        self.total_cost_usd = session.total_cost_usd;
        self.refresh_featured_assistant();
        self.reset_runtime();
    }

    fn clear_active_session_content(&mut self) {
        self.clear_pnl_card_attachment();
        self.input.clear();
        self.entries.clear();
        self.requested_model = None;
        self.runtime_model = None;
        self.context_tokens = None;
        self.context_window = None;
        self.total_tokens = None;
        self.total_cost_usd = None;
        self.stream.featured_entry_index = None;
        self.featured_response_has_image = false;
        self.reset_runtime();
    }

    fn allocate_session_id(&mut self, now_ms: u64) -> u64 {
        let id = self.next_session_id.max(now_ms).max(1);
        self.next_session_id = id.saturating_add(1);
        id
    }
}

fn latest_assistant_after_last_user(entries: &[AgentChatEntry]) -> Option<usize> {
    let turn_start = entries
        .iter()
        .rposition(|entry| {
            matches!(
                entry,
                AgentChatEntry::Message {
                    role: AgentChatRole::User,
                    ..
                }
            )
        })
        .unwrap_or_default();
    entries
        .iter()
        .enumerate()
        .skip(turn_start)
        .filter_map(|(index, entry)| {
            matches!(
                entry,
                AgentChatEntry::Message {
                    role: AgentChatRole::Assistant,
                    ..
                }
            )
            .then_some(index)
        })
        .next_back()
}

fn reveal_units_for_backlog(bytes: usize) -> usize {
    match bytes {
        0..=96 => 1,
        97..=256 => 2,
        257..=512 => 4,
        _ => 8,
    }
}

fn reveal_prefix_len(text: &str, max_units: usize, settled: bool) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut units = 0;
    let mut inside_unit = false;
    for (index, character) in text.char_indices() {
        if character.is_whitespace() {
            if inside_unit {
                units += 1;
                inside_unit = false;
            }
        } else {
            if units >= max_units {
                return index;
            }
            inside_unit = true;
        }
    }

    if settled || (!inside_unit && units > 0) {
        text.len()
    } else if units > 0 {
        text.char_indices()
            .rev()
            .find_map(|(index, character)| {
                character
                    .is_whitespace()
                    .then_some(index + character.len_utf8())
            })
            .unwrap_or_default()
    } else if text.chars().count() > 64 {
        text.char_indices()
            .nth(64)
            .map(|(index, _)| index)
            .unwrap_or(text.len())
    } else {
        0
    }
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn session_title(prompt: &str) -> String {
    let title = prompt
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("New session");
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = title.chars();
    let mut bounded = chars
        .by_ref()
        .take(MAX_SESSION_TITLE_CHARS)
        .collect::<String>();
    if chars.next().is_some() {
        let _ = bounded.pop();
        format!("{}…", bounded.trim_end())
    } else if bounded.is_empty() {
        "New session".to_string()
    } else {
        bounded
    }
}

fn message_count(entries: &[AgentChatEntry]) -> usize {
    entries
        .iter()
        .filter(|entry| matches!(entry, AgentChatEntry::Message { .. }))
        .count()
}

fn normalized_runtime_model(model: &str) -> Option<String> {
    let model = model.trim();
    if model.is_empty() {
        None
    } else {
        Some(bounded_text(model, MAX_RUNTIME_MODEL_CHARS))
    }
}

#[allow(clippy::too_many_arguments)]
fn persisted_session(
    id: u64,
    title: &str,
    created_at_ms: u64,
    updated_at_ms: u64,
    input: &str,
    entries: &[AgentChatEntry],
    requested_model: Option<&str>,
    runtime_model: Option<&str>,
    context_tokens: Option<u64>,
    context_window: Option<u64>,
    total_tokens: Option<u64>,
    total_cost_usd: Option<f64>,
) -> PersistedAgentSession {
    let entries = entries
        .iter()
        .filter_map(|entry| match entry {
            AgentChatEntry::Message { role, text, .. } if !text.is_empty() => {
                Some(PersistedAgentEntry {
                    role: match role {
                        AgentChatRole::User => PersistedAgentRole::User,
                        AgentChatRole::Assistant => PersistedAgentRole::Assistant,
                    },
                    text: bounded_text(text, MAX_PERSISTED_MESSAGE_CHARS),
                })
            }
            AgentChatEntry::Message { .. }
            | AgentChatEntry::Tool { .. }
            | AgentChatEntry::Reasoning { .. } => None,
        })
        .rev()
        .take(MAX_PERSISTED_ENTRIES_PER_SESSION)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    PersistedAgentSession {
        id,
        title: session_title(title),
        created_at_ms,
        updated_at_ms,
        input: bounded_text(input, MAX_PERSISTED_DRAFT_CHARS),
        entries,
        requested_model: requested_model.and_then(normalized_runtime_model),
        runtime_model: runtime_model.and_then(normalized_runtime_model),
        context_tokens,
        context_window: context_window.filter(|window| *window > 0),
        total_tokens,
        total_cost_usd: total_cost_usd.filter(|cost| cost.is_finite() && *cost >= 0.0),
    }
}

fn stored_session_from_persisted(session: PersistedAgentSession) -> AgentStoredSession {
    let entries = session
        .entries
        .into_iter()
        .rev()
        .take(MAX_PERSISTED_ENTRIES_PER_SESSION)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .filter_map(|entry| {
            let text = bounded_text(&entry.text, MAX_PERSISTED_MESSAGE_CHARS);
            if text.is_empty() {
                return None;
            }
            let role = match entry.role {
                PersistedAgentRole::User => AgentChatRole::User,
                PersistedAgentRole::Assistant => AgentChatRole::Assistant,
            };
            let markdown = (role == AgentChatRole::Assistant)
                .then(|| Box::new(markdown::Content::parse(&text)));
            Some(AgentChatEntry::Message {
                role,
                text,
                markdown,
                follow_ups: Vec::new(),
            })
        })
        .collect();
    AgentStoredSession {
        id: session.id.max(1),
        title: session_title(&session.title),
        created_at_ms: session.created_at_ms,
        updated_at_ms: session.updated_at_ms.max(session.created_at_ms),
        input: bounded_text(&session.input, MAX_PERSISTED_DRAFT_CHARS),
        entries,
        requested_model: session
            .requested_model
            .as_deref()
            .and_then(normalized_runtime_model),
        runtime_model: session
            .runtime_model
            .as_deref()
            .and_then(normalized_runtime_model),
        context_tokens: session.context_tokens,
        context_window: session.context_window.filter(|window| *window > 0),
        total_tokens: session.total_tokens,
        total_cost_usd: session
            .total_cost_usd
            .filter(|cost| cost.is_finite() && *cost >= 0.0),
    }
}

fn replay_transcript(entries: &[AgentChatEntry]) -> String {
    let mut remaining = MAX_REPLAY_CONTEXT_CHARS;
    let mut parts = Vec::new();
    for entry in entries.iter().rev() {
        let AgentChatEntry::Message { role, text, .. } = entry else {
            continue;
        };
        if remaining == 0 {
            break;
        }
        let label = match role {
            AgentChatRole::User => "user",
            AgentChatRole::Assistant => "assistant",
        };
        let overhead = label.len().saturating_mul(2).saturating_add(8);
        let available = remaining.saturating_sub(overhead);
        if available == 0 {
            break;
        }
        let text = trailing_text(text, available);
        remaining = remaining.saturating_sub(text.chars().count().saturating_add(overhead));
        parts.push(format!("<{label}>\n{text}\n</{label}>"));
    }
    parts.reverse();
    parts.join("\n\n")
}

fn split_assistant_follow_ups(response: &str) -> (String, Option<Vec<String>>) {
    let Some(section_start) = response.rfind(FOLLOW_UP_SECTION_START) else {
        return (response.to_string(), None);
    };
    if section_start > 0
        && !response[..section_start]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        return (response.to_string(), None);
    }

    let visible_answer = response[..section_start].trim_end().to_string();
    let metadata = &response[section_start + FOLLOW_UP_SECTION_START.len()..];
    let Some(section_end) = metadata.find(FOLLOW_UP_SECTION_END) else {
        return (visible_answer, Some(Vec::new()));
    };
    if !metadata[section_end + FOLLOW_UP_SECTION_END.len()..]
        .trim()
        .is_empty()
    {
        return (response.to_string(), None);
    }

    let payload = metadata[..section_end].trim();
    let parsed = serde_json::from_str::<Vec<String>>(payload).unwrap_or_default();
    let mut follow_ups = Vec::with_capacity(MAX_FOLLOW_UPS);
    for candidate in parsed {
        let normalized = candidate.split_whitespace().collect::<Vec<_>>().join(" ");
        let bounded = bounded_text(&normalized, MAX_FOLLOW_UP_CHARS);
        if bounded.is_empty()
            || follow_ups
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&bounded))
        {
            continue;
        }
        follow_ups.push(bounded);
        if follow_ups.len() == MAX_FOLLOW_UPS {
            break;
        }
    }
    (visible_answer, Some(follow_ups))
}

fn bounded_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn trailing_text(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars().rev().take(max_chars).collect::<Vec<_>>();
    chars.reverse();
    chars.into_iter().collect()
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
    fn response_metadata_becomes_personalized_follow_ups_not_visible_answer_text() {
        let mut state = AgentState::default();
        state.append_assistant_delta(
            "BTC concentration is the main risk.\n\n<!-- KEROSENE_FOLLOW_UPS_V1\n[\"How would a 5% BTC drop affect my current margin buffer?\",\"Which BTC position contributes most to the concentration?\"]\nKEROSENE_FOLLOW_UPS_V1 -->",
        );

        assert!(state.finalize_assistant_response_metadata());

        let [
            AgentChatEntry::Message {
                text,
                markdown: Some(_),
                follow_ups,
                ..
            },
        ] = state.entries.as_slice()
        else {
            panic!("expected one finalized assistant response");
        };
        assert_eq!(text, "BTC concentration is the main risk.");
        assert_eq!(
            follow_ups,
            &[
                "How would a 5% BTC drop affect my current margin buffer?".to_string(),
                "Which BTC position contributes most to the concentration?".to_string(),
            ]
        );
        assert!(!text.contains("KEROSENE_FOLLOW_UPS"));
        assert_eq!(state.persisted_store().sessions[0].entries[0].text, *text);

        assert!(state.finish_assistant_presentation().is_some());
        assert!(matches!(
            state.entries.as_slice(),
            [AgentChatEntry::Message { follow_ups, .. }] if follow_ups.len() == 2
        ));
    }

    #[test]
    fn malformed_or_absent_metadata_never_falls_back_to_generic_follow_ups() {
        let (plain, plain_follow_ups) = split_assistant_follow_ups("Visible answer");
        assert_eq!(plain, "Visible answer");
        assert!(plain_follow_ups.is_none());

        let (visible, malformed_follow_ups) =
            split_assistant_follow_ups("Visible answer\n<!-- KEROSENE_FOLLOW_UPS_V1\nnot-json");
        assert_eq!(visible, "Visible answer");
        assert!(malformed_follow_ups.is_some_and(|follow_ups| follow_ups.is_empty()));
    }

    #[test]
    fn follow_up_metadata_is_normalized_deduplicated_and_bounded() {
        let long_question = format!("{}?", "x".repeat(MAX_FOLLOW_UP_CHARS + 40));
        let response = format!(
            "Answer\n\n{FOLLOW_UP_SECTION_START}\n{}\n{FOLLOW_UP_SECTION_END}",
            serde_json::json!([
                "  Compare BTC   with ETH?  ",
                "compare btc with eth?",
                long_question,
                "This third unique question must be dropped?",
            ])
        );

        let (visible, follow_ups) = split_assistant_follow_ups(&response);
        let follow_ups = follow_ups.expect("metadata marker should be recognized");

        assert_eq!(visible, "Answer");
        assert_eq!(follow_ups.len(), MAX_FOLLOW_UPS);
        assert_eq!(follow_ups[0], "Compare BTC with ETH?");
        assert_eq!(follow_ups[1].chars().count(), MAX_FOLLOW_UP_CHARS);
    }

    #[test]
    fn streamed_reasoning_tracks_duration_and_stays_transient() {
        let mut state = AgentState {
            status: AgentStatus::Thinking,
            ..AgentState::default()
        };
        state.begin_reasoning();
        state.append_reasoning_delta("private portfolio reasoning");
        let _ = state.advance_assistant_stream();
        let _ = state.advance_assistant_stream();
        state.finish_reasoning();
        state.append_assistant_delta("Visible answer");

        let [reasoning, AgentChatEntry::Message { .. }] = state.entries.as_slice() else {
            panic!("expected a reasoning trace followed by the answer");
        };
        assert!(matches!(
            reasoning,
            AgentChatEntry::Reasoning {
                text,
                elapsed_ticks: 2,
                finished: true,
                expanded: true,
            } if text == "private portfolio reasoning"
        ));
        let debug = format!("{reasoning:?}");
        assert!(!debug.contains("private portfolio reasoning"));
        assert!(debug.contains("<redacted>"));

        let persisted = state.persisted_store();
        assert_eq!(persisted.sessions[0].entries.len(), 1);
        assert_eq!(persisted.sessions[0].entries[0].text, "Visible answer");
    }

    #[test]
    fn reasoning_disclosure_can_be_collapsed() {
        let mut state = AgentState::default();
        state.begin_reasoning();
        state.append_reasoning_delta("Trace");
        state.toggle_reasoning(0);

        assert!(matches!(
            state.entries.as_slice(),
            [AgentChatEntry::Reasoning {
                expanded: false,
                ..
            }]
        ));
    }

    #[test]
    fn chat_entry_debug_redacts_message_text() {
        let entry = AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "private portfolio question".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
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
        state.flush_assistant_stream();

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
    fn stream_reveal_preserves_exact_markdown_and_utf8() {
        let mut state = AgentState::default();
        let response = "## Risk\n\nBTC → ETH  **spread**\n";
        state.append_assistant_delta(response);

        for _ in 0..64 {
            let _ = state.advance_assistant_stream();
            if state.stream.pending.is_empty() {
                break;
            }
        }

        let Some(AgentChatEntry::Message {
            text,
            markdown: Some(markdown),
            ..
        }) = state.entries.first()
        else {
            panic!("expected streamed Assistant message");
        };
        assert_eq!(text, response);
        assert!(!markdown.items().is_empty());
        assert!(state.stream.pending.is_empty());
    }

    #[test]
    fn settled_partial_word_drains_and_completes() {
        let mut state = AgentState::default();
        state.append_assistant_delta("unfinished");

        let (changed, ready) = state.advance_assistant_stream();
        assert!(!changed);
        assert!(!ready);

        state.mark_assistant_transport_settled();
        let (changed, ready) = state.advance_assistant_stream();
        assert!(changed);
        assert!(ready);
        assert!(state.finish_assistant_presentation().is_some());
        assert!(state.assistant_entry_index.is_none());
    }

    #[test]
    fn busy_assistant_keeps_the_activity_animation_ticking() {
        let mut state = AgentState {
            status: AgentStatus::Preparing,
            ..AgentState::default()
        };

        assert!(state.stream_needs_tick());
        let _ = state.advance_assistant_stream();
        assert_eq!(state.stream.activity_ticks, 1);

        state.begin_snapshot(AgentPrompt::from("next turn".to_string()));
        assert_eq!(state.stream.activity_ticks, 0);
    }

    #[test]
    fn reveal_prefix_keeps_whitespace_and_waits_for_partial_words() {
        assert_eq!(reveal_prefix_len("hello world", 1, false), 6);
        assert_eq!(reveal_prefix_len("hello  \n\n", 1, false), 9);
        assert_eq!(reveal_prefix_len("partial", 1, false), 0);
        assert_eq!(reveal_prefix_len("partial", 1, true), 7);
        assert_eq!(reveal_prefix_len("éclair next", 1, false), 8);
    }

    #[test]
    fn agent_uri_debug_is_redacted() {
        let uri = AgentUri::from("https://example.com/private?token=secret".to_string());
        let debug = format!("{uri:?}");

        assert_eq!(debug, "AgentUri(<redacted>)");
        assert!(!debug.contains("token=secret"));
    }

    #[test]
    fn sessions_can_be_created_and_switched_without_losing_transcripts() {
        let mut state = AgentState::default();
        let first_id = state.active_session_id;
        state.note_user_prompt("Review my BTC risk", 10);
        state.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "Review my BTC risk".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
        });

        assert!(state.create_session(20));
        let second_id = state.active_session_id;
        assert_ne!(first_id, second_id);
        assert!(state.entries.is_empty());
        state.note_user_prompt("Show my best trades", 30);
        state.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "Show my best trades".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
        });

        assert!(state.switch_session(first_id));
        assert_eq!(state.active_session_id, first_id);
        assert_eq!(state.active_session_title, "Review my BTC risk");
        assert!(matches!(
            state.entries.as_slice(),
            [AgentChatEntry::Message { text, .. }] if text == "Review my BTC risk"
        ));
        assert!(state.needs_context_replay);
    }

    #[test]
    fn persisted_sessions_restore_markdown_and_active_selection() {
        let mut state = AgentState::default();
        state.note_user_prompt("Saved session", 10);
        state.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "private question".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
        });
        state.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text: "## Saved answer".to_string(),
            markdown: Some(Box::new(markdown::Content::parse("## Saved answer"))),
            follow_ups: Vec::new(),
        });
        state.prepare_context_for_model("openrouter/auto");
        state.update_runtime_model_context(Some("openrouter/auto".to_string()), Some(2_000_000));
        state.replace_context_usage(Some(12_000), Some(2_000_000));
        let active_id = state.active_session_id;

        let restored = AgentState::from_persisted_store(state.persisted_store());

        assert_eq!(restored.active_session_id, active_id);
        assert_eq!(restored.entries.len(), 2);
        assert!(matches!(
            &restored.entries[1],
            AgentChatEntry::Message {
                role: AgentChatRole::Assistant,
                markdown: Some(_),
                ..
            }
        ));
        assert!(restored.needs_context_replay);
        assert_eq!(
            restored.context_metrics_for_model("openrouter/auto"),
            (Some("openrouter/auto"), Some(12_000), Some(2_000_000))
        );
    }

    #[test]
    fn restored_session_runtime_prompt_replays_bounded_history() {
        let mut state = AgentState::default();
        state.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "Earlier private question".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
        });
        state.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text: "Earlier private answer".to_string(),
            markdown: Some(Box::new(markdown::Content::parse("Earlier private answer"))),
            follow_ups: Vec::new(),
        });
        state.needs_context_replay = true;

        let prompt = state.runtime_prompt("Follow up now");

        assert!(prompt.as_str().contains("Earlier private question"));
        assert!(prompt.as_str().contains("Earlier private answer"));
        assert!(prompt.as_str().contains("Follow up now"));
        assert_eq!(format!("{prompt:?}"), "AgentPrompt(<redacted>)");
    }

    #[test]
    fn stored_session_debug_redacts_titles_drafts_and_messages() {
        let session = AgentStoredSession {
            id: 1,
            title: "private title".to_string(),
            created_at_ms: 1,
            updated_at_ms: 2,
            input: "private draft".to_string(),
            entries: vec![AgentChatEntry::Message {
                role: AgentChatRole::User,
                text: "private message".to_string(),
                markdown: None,
                follow_ups: Vec::new(),
            }],
            requested_model: Some("openrouter/auto".to_string()),
            runtime_model: Some("openrouter/auto".to_string()),
            context_tokens: Some(1_024),
            context_window: Some(2_000_000),
            total_tokens: None,
            total_cost_usd: None,
        };

        let debug = format!("{session:?}");

        for private in ["private title", "private draft", "private message"] {
            assert!(!debug.contains(private));
        }
        assert!(!debug.contains("openrouter/auto"));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn generated_session_titles_respect_the_character_limit() {
        let title = session_title(&"a".repeat(MAX_SESSION_TITLE_CHARS + 10));

        assert_eq!(title.chars().count(), MAX_SESSION_TITLE_CHARS);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn persistence_result_debug_redacts_save_errors() {
        let result = AgentPersistenceResult::from(Err("private save detail".to_string()));
        let debug = format!("{result:?}");

        assert!(!debug.contains("private save detail"));
        assert_eq!(debug, "AgentPersistenceResult(Err(<redacted>))");
    }

    #[test]
    fn context_metrics_follow_each_session_and_reset_for_a_new_model() {
        let mut state = AgentState::default();
        let first_id = state.active_session_id;
        state.prepare_context_for_model("openrouter/auto");
        state.update_runtime_model_context(Some("openrouter/auto".to_string()), Some(2_000_000));
        state.replace_context_usage(Some(12_000), Some(2_000_000));

        assert!(state.create_session(20));
        state.prepare_context_for_model("anthropic/claude-sonnet-4.5");
        state.update_runtime_model_context(
            Some("anthropic/claude-sonnet-4.5".to_string()),
            Some(1_000_000),
        );
        state.replace_context_usage(Some(4_000), Some(1_000_000));

        assert!(state.switch_session(first_id));
        assert_eq!(
            state.context_metrics_for_model("openrouter/auto"),
            (Some("openrouter/auto"), Some(12_000), Some(2_000_000))
        );

        state.prepare_context_for_model("google/gemini-2.5-pro");
        assert_eq!(
            state.context_metrics_for_model("google/gemini-2.5-pro"),
            (None, None, None)
        );
    }

    #[test]
    fn legacy_persisted_sessions_default_context_metrics_to_unknown() {
        let session = serde_json::from_value::<PersistedAgentSession>(serde_json::json!({
            "id": 1,
            "title": "Legacy session",
            "created_at_ms": 1,
            "updated_at_ms": 2
        }))
        .expect("legacy Assistant session should deserialize");

        assert_eq!(session.requested_model, None);
        assert_eq!(session.runtime_model, None);
        assert_eq!(session.context_tokens, None);
        assert_eq!(session.context_window, None);
    }
}
