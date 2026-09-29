use crate::agent_pnl_card::AgentPnlCardAttachment;
use crate::llama_cpp::LlamaCppServer;
use crate::openrouter_api::{DEFAULT_OPENROUTER_MODEL, OpenRouterModel};
use iced::window;

mod model;
mod sessions;
mod stream;

pub(crate) use model::{
    AgentChatEntry, AgentChatRole, AgentPersistenceResult, AgentPrompt, AgentSessionListItem,
    AgentStatus, AgentStoredSession, AgentToolPresentation, AgentUri, PersistedAgentEntry,
    PersistedAgentRole, PersistedAgentSession, PersistedAgentStore,
};
pub(crate) use stream::{
    AGENT_PRESENTATION_TICK_MS, AgentStreamPresentation, agent_tool_presentation,
};

pub(crate) const MAX_AGENT_SESSIONS: usize = 50;

// ---------------------------------------------------------------------------
// Kerosene Assistant State
// ---------------------------------------------------------------------------

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
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn bounded_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}
