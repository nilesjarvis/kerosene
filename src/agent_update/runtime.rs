use crate::agent_runtime::{self, AgentRuntimeEvent};
use crate::agent_state::{AgentChatEntry, AgentPrompt, AgentStatus};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::Task;

// ---------------------------------------------------------------------------
// Assistant Runtime Events and Presentation
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn send_pending_agent_prompt(&mut self) -> Task<Message> {
        let Some(prompt) = self.agent.pending_prompt.take() else {
            self.agent.workspace_actions_allowed = false;
            self.agent.status = AgentStatus::Ready;
            self.agent.status_detail = None;
            return Task::none();
        };

        match agent_runtime::send_prompt(self.agent.runtime_generation, prompt) {
            Ok(()) => {
                self.agent.mark_context_replayed();
                self.agent.workspace_actions_allowed = true;
                self.agent.status = AgentStatus::Thinking;
                self.agent.status_detail = None;
            }
            Err(error) => {
                self.agent.workspace_actions_allowed = false;
                self.agent.runtime_connected = false;
                self.agent.require_context_replay();
                self.agent.status = AgentStatus::Error;
                self.agent.status_detail = Some(redact_sensitive_response_text(&error));
            }
        }
        Task::none()
    }

    pub(super) fn handle_agent_runtime_event(&mut self, event: AgentRuntimeEvent) -> Task<Message> {
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
            AgentRuntimeEvent::ReasoningStarted { .. } => {
                self.agent.begin_reasoning();
                self.agent.status = AgentStatus::Thinking;
                self.agent.status_detail = None;
                return self.snap_agent_chat_to_latest();
            }
            AgentRuntimeEvent::ReasoningDelta { delta, .. } => {
                self.agent.append_reasoning_delta(&delta);
                self.agent.status = AgentStatus::Thinking;
                self.agent.status_detail = None;
                return self.snap_agent_chat_to_latest();
            }
            AgentRuntimeEvent::ReasoningFinished { .. } => {
                self.agent.finish_reasoning();
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
                self.agent.status = AgentStatus::Thinking;
                if total_tokens.is_some() {
                    self.agent.total_tokens = total_tokens;
                }
                if total_cost_usd.is_some() {
                    self.agent.total_cost_usd = total_cost_usd;
                }
            }
            AgentRuntimeEvent::ToolStarted {
                call_id,
                name,
                detail,
                ..
            } => {
                self.agent.finish_reasoning();
                self.agent.flush_assistant_stream();
                self.agent.assistant_entry_index = None;
                let running_label =
                    crate::agent_state::agent_tool_presentation(&name).running_label;
                self.agent.entries.push(AgentChatEntry::Tool {
                    call_id,
                    name,
                    detail,
                    finished: false,
                    is_error: false,
                    expanded: true,
                });
                self.agent.status_detail = Some(format!("{running_label}…"));
                return self.snap_agent_chat_to_latest();
            }
            AgentRuntimeEvent::ToolFinished {
                call_id, is_error, ..
            } => {
                self.agent.finish_tool(&call_id, is_error);
                self.agent.status_detail = None;
            }
            AgentRuntimeEvent::ExtensionUiRequest {
                generation,
                request_id,
                method,
                title,
                payload,
            } => {
                if method == "input"
                    && !request_id.is_empty()
                    && title.as_deref() == Some(crate::agent_workspace::HOST_ACTION_RPC_TITLE)
                    && let Some(payload) = payload
                {
                    let (response, task) = self.handle_agent_host_action(&payload);
                    if let Err(error) = agent_runtime::respond_to_extension_ui(
                        generation,
                        request_id,
                        Some(response),
                    ) {
                        self.agent.workspace_actions_allowed = false;
                        self.agent.finish_running_tools(true);
                        self.agent.status = AgentStatus::Error;
                        self.agent.status_detail = Some(self.redact_agent_runtime_error(&error));
                    }
                    return task;
                }

                let _ = agent_runtime::respond_to_extension_ui(generation, request_id, None);
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
                self.agent.finish_reasoning();
                self.agent.finish_running_tools(true);
                if total_tokens.is_some() {
                    self.agent.total_tokens = total_tokens;
                }
                if total_cost_usd.is_some() {
                    self.agent.total_cost_usd = total_cost_usd;
                }

                let runtime_reported_visible_text = has_visible_text.unwrap_or(false);
                let has_visible_text = self.agent.finalize_assistant_response_metadata();
                match empty_response_action(
                    has_visible_text,
                    self.agent.suppress_empty_response_retry,
                    self.agent.empty_response_retry_count,
                ) {
                    EmptyResponseAction::Retry => {
                        self.agent.flush_assistant_stream();
                        self.agent.empty_response_retry_count = 1;
                        self.agent.current_turn_has_text = false;
                        self.agent.assistant_entry_index = None;
                        self.agent.status = AgentStatus::Thinking;
                        self.agent.status_detail = Some(
                            if runtime_reported_visible_text {
                                "Pi returned response metadata without an answer; retrying once…"
                            } else {
                                "Pi returned no visible text; retrying once…"
                            }
                            .to_string(),
                        );
                        let retry = AgentPrompt::from(
                            "Your previous turn returned no visible answer text. Provide a concise, complete answer to the user's immediately preceding request now. Reuse any tool results already gathered, call only a missing narrow tool if essential, and finish with visible Markdown text."
                                .to_string(),
                        );
                        if let Err(error) =
                            agent_runtime::send_prompt(self.agent.runtime_generation, retry)
                        {
                            self.agent.workspace_actions_allowed = false;
                            self.agent.runtime_connected = false;
                            self.agent.require_context_replay();
                            self.agent.status = AgentStatus::Error;
                            self.agent.status_detail = Some(redact_sensitive_response_text(&error));
                        }
                        return Task::none();
                    }
                    EmptyResponseAction::Error => {
                        self.agent.workspace_actions_allowed = false;
                        self.agent.feature_latest_assistant_immediately();
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

                self.agent.workspace_actions_allowed = false;
                self.agent.suppress_empty_response_retry = false;
                self.agent.mark_assistant_transport_settled();
                if self.agent.assistant_stream_ready_to_finalize() {
                    return self.finish_agent_turn_presentation();
                }
                self.agent.status = AgentStatus::Thinking;
                self.agent.status_detail = None;
                return Task::none();
            }
            AgentRuntimeEvent::Error { message, .. } => {
                self.agent.workspace_actions_allowed = false;
                self.agent.finish_running_tools(true);
                self.agent.finish_reasoning();
                self.agent.pending_prompt = None;
                self.agent.feature_latest_assistant_immediately();
                self.agent.status = AgentStatus::Error;
                self.agent.status_detail = Some(self.redact_agent_runtime_error(&message));
                self.agent.assistant_entry_index = None;
                self.agent.mark_active_session_updated(Self::now_ms());
                return self.persist_agent_sessions();
            }
            AgentRuntimeEvent::Exited { .. } => {
                self.agent.workspace_actions_allowed = false;
                self.agent.finish_running_tools(true);
                self.agent.finish_reasoning();
                self.agent.flush_assistant_stream();
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

    pub(super) fn advance_agent_stream_presentation(&mut self) -> Task<Message> {
        let (visible_changed, ready_to_finalize) = self.agent.advance_assistant_stream();
        if ready_to_finalize {
            let finish = self.finish_agent_turn_presentation();
            return if visible_changed {
                Task::batch([self.snap_agent_chat_to_latest(), finish])
            } else {
                finish
            };
        }
        if visible_changed {
            self.snap_agent_chat_to_latest()
        } else {
            Task::none()
        }
    }

    fn finish_agent_turn_presentation(&mut self) -> Task<Message> {
        self.agent.workspace_actions_allowed = false;
        self.agent.finish_assistant_presentation();
        self.agent.status = AgentStatus::Ready;
        self.agent.status_detail = None;
        self.agent.suppress_empty_response_retry = false;
        self.agent.mark_active_session_updated(Self::now_ms());
        let _ = agent_runtime::inspect_context(self.agent.runtime_generation);
        Task::batch([
            self.persist_agent_sessions(),
            self.snap_agent_chat_to_latest(),
        ])
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

#[cfg(test)]
mod tests {
    use super::*;

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
