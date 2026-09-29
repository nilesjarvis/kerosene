use crate::agent_persistence;
use crate::agent_runtime;
use crate::agent_snapshot;
use crate::agent_state::AgentState;
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::{Task, window};

// ---------------------------------------------------------------------------
// Assistant Sessions and Runtime Cleanup
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn create_agent_session(&mut self) -> Task<Message> {
        if self.agent.status.is_busy() {
            self.agent.status_detail =
                Some("Stop the current response before creating a session.".to_string());
            return Task::none();
        }
        let generation = self.agent.runtime_generation;
        let request_id = self.agent.snapshot_request_id;
        if !self.agent.create_session(Self::now_ms()) {
            return Task::none();
        }
        self.shutdown_agent_runtime_files(generation, request_id);
        Task::batch([
            self.persist_agent_sessions(),
            self.snap_agent_chat_to_latest(),
        ])
    }

    pub(super) fn select_agent_session(&mut self, id: u64) -> Task<Message> {
        if self.agent.status.is_busy() {
            self.agent.status_detail =
                Some("Stop the current response before switching sessions.".to_string());
            return Task::none();
        }
        let generation = self.agent.runtime_generation;
        let request_id = self.agent.snapshot_request_id;
        if !self.agent.switch_session(id) {
            return Task::none();
        }
        self.shutdown_agent_runtime_files(generation, request_id);
        Task::batch([
            self.persist_agent_sessions(),
            self.snap_agent_chat_to_latest(),
        ])
    }

    pub(super) fn shutdown_agent_runtime_files(&self, generation: u64, request_id: u64) {
        agent_runtime::shutdown(generation);
        agent_snapshot::clear_sensitive_runtime_files(
            &agent_snapshot::workspace_dir(),
            generation,
            request_id,
        );
    }

    pub(super) fn persist_agent_sessions(&mut self) -> Task<Message> {
        if self.config_clear_requested || self.config_cleared_this_session {
            self.agent.persistence_dirty = false;
            return Task::none();
        }
        if self.agent.persistence_in_flight {
            self.agent.persistence_dirty = true;
            return Task::none();
        }
        self.agent.persistence_generation = self.agent.persistence_generation.wrapping_add(1);
        let generation = self.agent.persistence_generation;
        self.agent.persistence_in_flight = true;
        self.agent.persistence_dirty = false;
        let store = self.agent.persisted_store();
        Task::perform(agent_persistence::save_agent_store(store), move |result| {
            Message::AgentSessionsSaved(generation, result.into())
        })
    }

    pub(super) fn handle_agent_sessions_saved(
        &mut self,
        generation: u64,
        result: Result<(), String>,
    ) -> Task<Message> {
        if generation != self.agent.persistence_generation {
            return Task::none();
        }
        self.agent.persistence_in_flight = false;
        self.agent.persistence_error = result.err().map(|error| {
            redact_sensitive_response_text(&format!("Could not save Assistant sessions: {error}"))
        });
        if self.config_clear_requested && !self.config_cleared_this_session {
            self.agent.persistence_dirty = false;
            return self.start_config_clear_task();
        }
        if self.agent.persistence_dirty {
            return self.persist_agent_sessions();
        }
        Task::none()
    }

    pub(crate) fn invalidate_agent_runtime(&mut self) {
        let generation = self.agent.runtime_generation;
        let request_id = self.agent.snapshot_request_id;
        self.shutdown_agent_runtime_files(generation, request_id);
        self.agent.reset_runtime();
    }

    pub(crate) fn prepare_agent_for_config_clear(&mut self) -> Task<Message> {
        let runtime_generation = self.agent.runtime_generation;
        let snapshot_request_id = self.agent.snapshot_request_id;
        let next_runtime_generation = runtime_generation.wrapping_add(1);
        let next_snapshot_request_id = snapshot_request_id.wrapping_add(1);
        let persistence_generation = self.agent.persistence_generation;
        let persistence_in_flight = self.agent.persistence_in_flight;
        let window_id = self.agent.window_id;

        self.shutdown_agent_runtime_files(runtime_generation, snapshot_request_id);

        let mut cleared = AgentState {
            runtime_generation: next_runtime_generation,
            snapshot_request_id: next_snapshot_request_id,
            persistence_generation,
            persistence_in_flight,
            ..AgentState::default()
        };
        cleared.persistence_dirty = false;
        self.agent = cleared;

        window_id.map_or_else(Task::none, window::close)
    }

    pub(crate) fn close_agent_session(&mut self) {
        self.invalidate_agent_runtime();
        self.agent.model_picker_open = false;
        self.agent.model_search.clear();
        self.agent.clear_pnl_card_attachment();
        self.agent.window_id = None;
        if self.config_clear_requested || self.config_cleared_this_session {
            self.agent.persistence_dirty = false;
            return;
        }
        if let Err(error) = agent_persistence::save_agent_store_now(&self.agent.persisted_store()) {
            self.agent.persistence_error = Some(redact_sensitive_response_text(&format!(
                "Could not save Assistant sessions: {error}"
            )));
        }
    }
}
