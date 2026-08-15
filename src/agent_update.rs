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
            Message::AgentNewChat => {
                let generation = self.agent.runtime_generation;
                let request_id = self.agent.snapshot_request_id;
                let workspace_dir = agent_snapshot::workspace_dir();
                agent_runtime::shutdown(generation);
                agent_snapshot::clear_sensitive_runtime_files(
                    &workspace_dir,
                    generation,
                    request_id,
                );
                self.agent.reset_session();
                Task::none()
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
            return window::gain_focus(id);
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
            size: Size::new(760.0, 720.0),
            min_size: Some(Size::new(520.0, 480.0)),
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (id, task) = window::open(settings);
        self.agent.window_id = Some(id);
        task.map(Message::WindowOpened)
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
        let (generation, request_id) = self.agent.begin_snapshot(prompt.into());
        let workspace_dir = agent_snapshot::workspace_dir();

        Task::perform(
            agent_snapshot::write_agent_snapshot(workspace_dir, generation, request_id, snapshot),
            move |result| Message::AgentSnapshotPrepared(generation, request_id, result),
        )
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
                self.agent.status = AgentStatus::Thinking;
                self.agent.status_detail = None;
            }
            Err(error) => {
                self.agent.runtime_connected = false;
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
            AgentRuntimeEvent::Ready { .. } => {
                self.agent.runtime_connected = true;
                self.agent.status = AgentStatus::Ready;
                self.agent.status_detail = None;
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
                        return Task::none();
                    }
                    EmptyResponseAction::Accept => {}
                }

                self.agent.status = AgentStatus::Ready;
                self.agent.status_detail = None;
                self.agent.assistant_entry_index = None;
                self.agent.suppress_empty_response_retry = false;
            }
            AgentRuntimeEvent::Error { message, .. } => {
                self.agent.pending_prompt = None;
                self.agent.status = AgentStatus::Error;
                self.agent.status_detail = Some(self.redact_agent_runtime_error(&message));
                self.agent.assistant_entry_index = None;
            }
            AgentRuntimeEvent::Exited { .. } => {
                self.agent.runtime_connected = false;
                if self.agent.status != AgentStatus::Error
                    && self.agent.status != AgentStatus::Stopped
                {
                    self.agent.status = AgentStatus::Error;
                    self.agent.status_detail = Some("Pi stopped unexpectedly.".to_string());
                }
            }
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
        agent_runtime::shutdown(generation);
        agent_snapshot::clear_sensitive_runtime_files(
            &agent_snapshot::workspace_dir(),
            generation,
            request_id,
        );
        self.agent.reset_session();
    }

    pub(crate) fn close_agent_session(&mut self) {
        let generation = self.agent.runtime_generation;
        let request_id = self.agent.snapshot_request_id;
        let workspace_dir = agent_snapshot::workspace_dir();
        agent_runtime::shutdown(generation);
        agent_snapshot::clear_sensitive_runtime_files(&workspace_dir, generation, request_id);
        self.agent.reset_session();
        self.agent.window_id = None;
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
}
