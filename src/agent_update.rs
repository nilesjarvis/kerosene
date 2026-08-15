use crate::agent_persistence;
use crate::agent_runtime::{self, AgentRuntimeConfig, AgentRuntimeEvent};
use crate::agent_snapshot;
use crate::agent_state::{AgentChatEntry, AgentChatRole, AgentPrompt, AgentStatus};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::{Size, Task, window};
#[cfg(not(target_os = "windows"))]
use std::process::Command;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

// ---------------------------------------------------------------------------
// Kerosene Assistant Update
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn update_agent(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenAgentWindow => self.open_agent_window(),
            Message::AgentInputChanged(input) => {
                self.agent.input = input.into_string();
                Task::none()
            }
            Message::AgentSubmit => self.submit_agent_prompt(),
            Message::AgentSnapshotPrepared(generation, request_id, result) => {
                self.handle_agent_snapshot_prepared(generation, request_id, result)
            }
            Message::AgentRuntimeEvent(event) => self.handle_agent_runtime_event(event),
            Message::AgentAbort => {
                self.agent.suppress_empty_response_retry = true;
                agent_runtime::abort(self.agent.runtime_generation);
                self.agent.status_detail = Some("Stopping the current response…".to_string());
                Task::none()
            }
            Message::AgentNewChat => self.create_agent_session(),
            Message::AgentSelectSession(id) => self.select_agent_session(id),
            Message::AgentSessionsSaved(generation, result) => {
                self.handle_agent_sessions_saved(generation, result.into_result())
            }
            Message::AgentOpenLink(uri) => {
                let uri = uri.into_string().trim().to_string();
                if !agent_link_is_allowed(&uri) {
                    self.agent.status_detail =
                        Some("Only HTTP and HTTPS Assistant links can be opened.".to_string());
                    return Task::none();
                }
                Task::perform(open_agent_link(uri), Message::AgentLinkOpened)
            }
            Message::AgentLinkOpened(result) => {
                self.agent.status_detail = result.err().map(|error| {
                    redact_sensitive_response_text(&format!(
                        "Could not open the Assistant link: {error}"
                    ))
                });
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn open_agent_window(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        self.layout_menu_open = false;
        self.account_picker_open = false;
        self.account_picker_rename_index = None;

        if let Some(id) = self.agent.window_id {
            let focus = window::gain_focus(id);
            let journal = if self.connected_address.is_some() {
                self.load_journal_for_active_account(false)
            } else {
                Task::none()
            };
            return Task::batch([focus, journal]);
        }

        if !self.openrouter_configured() {
            self.agent.status = AgentStatus::Error;
            self.agent.status_detail = Some(
                "Add an OpenRouter API key in Settings → Integrations to start chatting."
                    .to_string(),
            );
        } else if self.agent.status == AgentStatus::Error {
            self.agent.status = AgentStatus::Stopped;
            self.agent.status_detail = None;
        }

        let settings = window::Settings {
            size: Size::new(940.0, 720.0),
            min_size: Some(Size::new(720.0, 480.0)),
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (id, task) = window::open(settings);
        self.agent.window_id = Some(id);
        let journal = if self.connected_address.is_some() {
            self.load_journal_for_active_account(false)
        } else {
            Task::none()
        };
        Task::batch([task.map(Message::WindowOpened), journal])
    }

    fn submit_agent_prompt(&mut self) -> Task<Message> {
        if self.agent.status.is_busy() {
            return Task::none();
        }

        let prompt = self.agent.input.trim().to_string();
        if prompt.is_empty() {
            return Task::none();
        }
        if !self.openrouter_configured() {
            self.agent.status = AgentStatus::Error;
            self.agent.status_detail = Some(
                "Add an OpenRouter API key in Settings → Integrations before sending.".to_string(),
            );
            return Task::none();
        }

        let snapshot = match self.build_agent_snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.agent.status = AgentStatus::Error;
                self.agent.status_detail = Some(error);
                return Task::none();
            }
        };

        let model = self.openrouter_model_for_task();
        self.agent.prepare_context_for_model(&model);
        let runtime_prompt = self.agent.runtime_prompt(&prompt);
        self.agent.note_user_prompt(&prompt, Self::now_ms());
        self.agent.input.clear();
        self.agent.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: prompt.clone(),
            markdown: None,
        });
        self.agent.assistant_entry_index = None;
        if !self.agent.runtime_connected {
            self.agent.begin_new_runtime();
        }
        let (generation, request_id) = self.agent.begin_snapshot(runtime_prompt);
        let workspace_dir = agent_snapshot::workspace_dir();

        let snapshot_task = Task::perform(
            agent_snapshot::write_agent_snapshot(workspace_dir, generation, request_id, snapshot),
            move |result| Message::AgentSnapshotPrepared(generation, request_id, result),
        );
        Task::batch([snapshot_task, self.persist_agent_sessions()])
    }

    fn handle_agent_snapshot_prepared(
        &mut self,
        generation: u64,
        request_id: u64,
        result: Result<std::path::PathBuf, String>,
    ) -> Task<Message> {
        if generation != self.agent.runtime_generation
            || request_id != self.agent.snapshot_request_id
        {
            if let Ok(path) = result {
                let _ = std::fs::remove_file(path);
            }
            return Task::none();
        }

        let staged_path = match result {
            Ok(path) => path,
            Err(error) => {
                self.agent.pending_prompt = None;
                self.agent.status = AgentStatus::Error;
                self.agent.status_detail = Some(redact_sensitive_response_text(&error));
                return Task::none();
            }
        };
        let workspace_dir = staged_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(agent_snapshot::workspace_dir);
        if let Err(error) = agent_snapshot::activate_agent_snapshot(&workspace_dir, &staged_path) {
            self.agent.pending_prompt = None;
            self.agent.status = AgentStatus::Error;
            self.agent.status_detail = Some(redact_sensitive_response_text(&error));
            return Task::none();
        }

        if self.agent.runtime_connected {
            return self.send_pending_agent_prompt();
        }

        self.agent.status = AgentStatus::Starting;
        self.agent.status_detail = Some("Launching the local Pi RPC harness…".to_string());
        let config = AgentRuntimeConfig {
            generation,
            model: self.openrouter_model_for_task(),
            api_key: self.openrouter_api_key_for_task(),
            hyperdash_api_key: self.hyperdash_api_key_for_task(),
            workspace_dir,
        };

        Task::run(agent_runtime::runtime_stream(config), |event| {
            Message::AgentRuntimeEvent(event)
        })
    }

    fn send_pending_agent_prompt(&mut self) -> Task<Message> {
        let Some(prompt) = self.agent.pending_prompt.take() else {
            self.agent.status = AgentStatus::Ready;
            self.agent.status_detail = None;
            return Task::none();
        };

        match agent_runtime::send_prompt(self.agent.runtime_generation, prompt) {
            Ok(()) => {
                self.agent.mark_context_replayed();
                self.agent.status = AgentStatus::Thinking;
                self.agent.status_detail = None;
            }
            Err(error) => {
                self.agent.runtime_connected = false;
                self.agent.require_context_replay();
                self.agent.status = AgentStatus::Error;
                self.agent.status_detail = Some(redact_sensitive_response_text(&error));
            }
        }
        Task::none()
    }

    fn handle_agent_runtime_event(&mut self, event: AgentRuntimeEvent) -> Task<Message> {
        if event.generation() != self.agent.runtime_generation {
            return Task::none();
        }

        match event {
            AgentRuntimeEvent::Ready { generation } => {
                self.agent.runtime_connected = true;
                self.agent.status = AgentStatus::Ready;
                self.agent.status_detail = None;
                let _ = agent_runtime::inspect_context(generation);
                return self.send_pending_agent_prompt();
            }
            AgentRuntimeEvent::Thinking { .. } => {
                self.agent.status = AgentStatus::Thinking;
                self.agent.status_detail = None;
            }
            AgentRuntimeEvent::TextDelta {
                delta,
                total_tokens,
                total_cost_usd,
                ..
            } => {
                self.agent.append_assistant_delta(&delta);
                if total_tokens.is_some() {
                    self.agent.total_tokens = total_tokens;
                }
                if total_cost_usd.is_some() {
                    self.agent.total_cost_usd = total_cost_usd;
                }
                return self.snap_agent_chat_to_latest();
            }
            AgentRuntimeEvent::ToolStarted { call_id, name, .. } => {
                self.agent.assistant_entry_index = None;
                self.agent.entries.push(AgentChatEntry::Tool {
                    call_id,
                    name,
                    finished: false,
                    is_error: false,
                });
                self.agent.status_detail = Some("Reading Kerosene data…".to_string());
            }
            AgentRuntimeEvent::ToolFinished {
                call_id, is_error, ..
            } => {
                self.agent.finish_tool(&call_id, is_error);
                self.agent.status_detail = None;
            }
            AgentRuntimeEvent::ModelContext {
                model,
                context_window,
                ..
            } => {
                self.agent
                    .update_runtime_model_context(model, context_window);
                return self.persist_agent_sessions();
            }
            AgentRuntimeEvent::ContextUsage {
                context_tokens,
                context_window,
                ..
            } => {
                self.agent
                    .replace_context_usage(context_tokens, context_window);
                return self.persist_agent_sessions();
            }
            AgentRuntimeEvent::Settled {
                total_tokens,
                total_cost_usd,
                has_visible_text,
                ..
            } => {
                if total_tokens.is_some() {
                    self.agent.total_tokens = total_tokens;
                }
                if total_cost_usd.is_some() {
                    self.agent.total_cost_usd = total_cost_usd;
                }

                let has_visible_text = has_visible_text.unwrap_or(self.agent.current_turn_has_text)
                    || self.agent.current_turn_has_text;
                match empty_response_action(
                    has_visible_text,
                    self.agent.suppress_empty_response_retry,
                    self.agent.empty_response_retry_count,
                ) {
                    EmptyResponseAction::Retry => {
                        self.agent.empty_response_retry_count = 1;
                        self.agent.current_turn_has_text = false;
                        self.agent.assistant_entry_index = None;
                        self.agent.status = AgentStatus::Thinking;
                        self.agent.status_detail =
                            Some("Pi returned no visible text; retrying once…".to_string());
                        let retry = AgentPrompt::from(
                            "Your previous turn returned no visible answer text. Provide a concise, complete answer to the user's immediately preceding request now. Reuse any tool results already gathered, call only a missing narrow tool if essential, and finish with visible Markdown text."
                                .to_string(),
                        );
                        if let Err(error) =
                            agent_runtime::send_prompt(self.agent.runtime_generation, retry)
                        {
                            self.agent.runtime_connected = false;
                            self.agent.require_context_replay();
                            self.agent.status = AgentStatus::Error;
                            self.agent.status_detail = Some(redact_sensitive_response_text(&error));
                        }
                        return Task::none();
                    }
                    EmptyResponseAction::Error => {
                        self.agent.status = AgentStatus::Error;
                        self.agent.status_detail = Some(
                            "Pi completed twice without visible answer text. Try a shorter prompt or another model."
                                .to_string(),
                        );
                        self.agent.assistant_entry_index = None;
                        self.agent.mark_active_session_updated(Self::now_ms());
                        let _ = agent_runtime::inspect_context(self.agent.runtime_generation);
                        return self.persist_agent_sessions();
                    }
                    EmptyResponseAction::Accept => {}
                }

                self.agent.status = AgentStatus::Ready;
                self.agent.status_detail = None;
                self.agent.assistant_entry_index = None;
                self.agent.suppress_empty_response_retry = false;
                self.agent.mark_active_session_updated(Self::now_ms());
                let _ = agent_runtime::inspect_context(self.agent.runtime_generation);
                return self.persist_agent_sessions();
            }
            AgentRuntimeEvent::Error { message, .. } => {
                self.agent.pending_prompt = None;
                self.agent.status = AgentStatus::Error;
                self.agent.status_detail = Some(self.redact_agent_runtime_error(&message));
                self.agent.assistant_entry_index = None;
                self.agent.mark_active_session_updated(Self::now_ms());
                return self.persist_agent_sessions();
            }
            AgentRuntimeEvent::Exited { .. } => {
                self.agent.runtime_connected = false;
                self.agent.require_context_replay();
                if self.agent.status != AgentStatus::Error
                    && self.agent.status != AgentStatus::Stopped
                {
                    self.agent.status = AgentStatus::Error;
                    self.agent.status_detail = Some("Pi stopped unexpectedly.".to_string());
                }
                if self.agent.current_turn_has_text {
                    self.agent.mark_active_session_updated(Self::now_ms());
                    return self.persist_agent_sessions();
                }
            }
        }
        Task::none()
    }

    fn create_agent_session(&mut self) -> Task<Message> {
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

    fn select_agent_session(&mut self, id: u64) -> Task<Message> {
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

    fn shutdown_agent_runtime_files(&self, generation: u64, request_id: u64) {
        agent_runtime::shutdown(generation);
        agent_snapshot::clear_sensitive_runtime_files(
            &agent_snapshot::workspace_dir(),
            generation,
            request_id,
        );
    }

    fn persist_agent_sessions(&mut self) -> Task<Message> {
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

    fn handle_agent_sessions_saved(
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
        if self.agent.persistence_dirty {
            return self.persist_agent_sessions();
        }
        Task::none()
    }

    fn snap_agent_chat_to_latest(&self) -> Task<Message> {
        iced::widget::operation::snap_to(
            iced::widget::Id::new("kerosene-agent-chat"),
            iced::widget::scrollable::RelativeOffset::END,
        )
    }

    fn redact_agent_runtime_error(&self, message: &str) -> String {
        let mut redacted = redact_sensitive_response_text(message);
        for key in [
            self.openrouter_api_key.trim(),
            self.hyperdash_api_key.trim(),
        ] {
            if !key.is_empty() {
                redacted = redacted.replace(key, "<redacted>");
            }
        }
        const MAX_ERROR_CHARS: usize = 600;
        let mut chars = redacted.chars();
        let bounded = chars.by_ref().take(MAX_ERROR_CHARS).collect::<String>();
        if chars.next().is_some() {
            format!("{bounded}…")
        } else {
            bounded
        }
    }

    pub(crate) fn invalidate_agent_runtime(&mut self) {
        let generation = self.agent.runtime_generation;
        let request_id = self.agent.snapshot_request_id;
        self.shutdown_agent_runtime_files(generation, request_id);
        self.agent.reset_runtime();
    }

    pub(crate) fn close_agent_session(&mut self) {
        let generation = self.agent.runtime_generation;
        let request_id = self.agent.snapshot_request_id;
        self.shutdown_agent_runtime_files(generation, request_id);
        self.agent.reset_runtime();
        self.agent.window_id = None;
        if let Err(error) = agent_persistence::save_agent_store_now(&self.agent.persisted_store()) {
            self.agent.persistence_error = Some(redact_sensitive_response_text(&format!(
                "Could not save Assistant sessions: {error}"
            )));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmptyResponseAction {
    Accept,
    Retry,
    Error,
}

fn empty_response_action(
    has_visible_text: bool,
    suppress_retry: bool,
    retry_count: u8,
) -> EmptyResponseAction {
    if has_visible_text || suppress_retry {
        EmptyResponseAction::Accept
    } else if retry_count == 0 {
        EmptyResponseAction::Retry
    } else {
        EmptyResponseAction::Error
    }
}

fn agent_link_is_allowed(uri: &str) -> bool {
    let uri = uri.to_ascii_lowercase();
    uri.starts_with("https://") || uri.starts_with("http://")
}

async fn open_agent_link(uri: String) -> Result<(), String> {
    open_agent_link_with_system(&uri)
}

#[cfg(target_os = "macos")]
fn open_agent_link_with_system(uri: &str) -> Result<(), String> {
    Command::new("open")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("open command failed: {error}"))
}

#[cfg(target_os = "windows")]
fn open_agent_link_with_system(uri: &str) -> Result<(), String> {
    let operation = "open\0".encode_utf16().collect::<Vec<_>>();
    let uri = uri
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            uri.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    let status = result as isize;
    if status > 32 {
        Ok(())
    } else {
        Err(format!("Windows URL launch failed with status {status}"))
    }
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_agent_link_with_system(uri: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("xdg-open command failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_requires_an_openrouter_key() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.openrouter_api_key.clear();
        terminal.agent.input = "Analyze my risk".to_string();

        let _ = terminal.update_agent(Message::AgentSubmit);

        assert_eq!(terminal.agent.status, AgentStatus::Error);
        assert!(terminal.agent.entries.is_empty());
    }

    #[test]
    fn settled_runtime_event_returns_ready_and_updates_usage() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.agent.runtime_generation = 4;
        terminal.agent.status = AgentStatus::Thinking;

        let _ = terminal.update_agent(Message::AgentRuntimeEvent(AgentRuntimeEvent::Settled {
            generation: 4,
            total_tokens: Some(123),
            total_cost_usd: Some(0.0042),
            has_visible_text: Some(true),
        }));

        assert_eq!(terminal.agent.status, AgentStatus::Ready);
        assert_eq!(terminal.agent.total_tokens, Some(123));
        assert_eq!(terminal.agent.total_cost_usd, Some(0.0042));
    }

    #[test]
    fn assistant_links_only_allow_http_schemes() {
        assert!(agent_link_is_allowed("https://example.com/report"));
        assert!(agent_link_is_allowed("HTTP://example.com/report"));
        assert!(!agent_link_is_allowed("file:///tmp/private"));
        assert!(!agent_link_is_allowed("javascript:alert(1)"));
    }

    #[test]
    fn empty_response_retries_once_but_not_after_abort() {
        assert_eq!(
            empty_response_action(false, false, 0),
            EmptyResponseAction::Retry
        );
        assert_eq!(
            empty_response_action(false, false, 1),
            EmptyResponseAction::Error
        );
        assert_eq!(
            empty_response_action(false, true, 0),
            EmptyResponseAction::Accept
        );
        assert_eq!(
            empty_response_action(true, false, 0),
            EmptyResponseAction::Accept
        );
    }

    #[test]
    fn new_chat_creates_a_saved_session_instead_of_erasing_the_previous_one() {
        let (mut terminal, _) = TradingTerminal::boot();
        let first_id = terminal.agent.active_session_id;
        terminal.agent.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "private first session".to_string(),
            markdown: None,
        });

        let _ = terminal.update_agent(Message::AgentNewChat);

        assert_ne!(terminal.agent.active_session_id, first_id);
        assert!(terminal.agent.entries.is_empty());
        assert_eq!(terminal.agent.sessions.len(), 1);
        assert!(matches!(
            terminal.agent.sessions[0].entries.as_slice(),
            [AgentChatEntry::Message { text, .. }] if text == "private first session"
        ));
    }

    #[test]
    fn closing_assistant_preserves_active_transcript() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.agent.window_id = Some(window::Id::unique());
        terminal.agent.entries.push(AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text: "saved answer".to_string(),
            markdown: Some(Box::new(iced::widget::markdown::Content::parse(
                "saved answer",
            ))),
        });

        terminal.close_agent_session();

        assert!(terminal.agent.window_id.is_none());
        assert_eq!(terminal.agent.entries.len(), 1);
        assert!(terminal.agent.needs_context_replay);
    }

    #[test]
    fn runtime_context_updates_are_scoped_to_the_active_generation() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.agent.runtime_generation = 4;
        terminal
            .agent
            .prepare_context_for_model("anthropic/claude-sonnet-4.5");

        let _ = terminal.update_agent(Message::AgentRuntimeEvent(
            AgentRuntimeEvent::ModelContext {
                generation: 4,
                model: Some("anthropic/claude-sonnet-4.5".to_string()),
                context_window: Some(1_000_000),
            },
        ));
        let _ = terminal.update_agent(Message::AgentRuntimeEvent(
            AgentRuntimeEvent::ContextUsage {
                generation: 4,
                context_tokens: Some(25_000),
                context_window: Some(1_000_000),
            },
        ));

        assert_eq!(terminal.agent.context_tokens, Some(25_000));
        assert_eq!(terminal.agent.context_window, Some(1_000_000));

        let _ = terminal.update_agent(Message::AgentRuntimeEvent(
            AgentRuntimeEvent::ContextUsage {
                generation: 3,
                context_tokens: Some(999_999),
                context_window: Some(1_000_000),
            },
        ));
        assert_eq!(terminal.agent.context_tokens, Some(25_000));

        let _ = terminal.update_agent(Message::AgentRuntimeEvent(
            AgentRuntimeEvent::ContextUsage {
                generation: 4,
                context_tokens: None,
                context_window: Some(1_000_000),
            },
        ));
        assert_eq!(terminal.agent.context_tokens, None);
    }
}
