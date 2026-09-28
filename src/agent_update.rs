use crate::agent_runtime;
use crate::agent_state::{AgentChatEntry, AgentChatRole, AgentStatus};
use crate::app_state::TradingTerminal;
use crate::config::AssistantProvider;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::{Size, Task, window};

mod links;
mod prompts;
mod providers;
mod runtime;
mod sessions;

use self::links::{agent_link_is_allowed, open_agent_link};

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
            Message::AgentPnlCardBrowse => self.browse_agent_pnl_card(),
            Message::AgentPnlCardDropped(window_id, path) => {
                if self.agent.window_id != Some(window_id) {
                    return Task::none();
                }
                self.load_dropped_agent_pnl_card(path.into_path_buf())
            }
            Message::AgentPnlCardHoverChanged(window_id, hovered) => {
                if self.agent.window_id == Some(window_id) && !self.agent.status.is_busy() {
                    self.agent.pnl_card_drop_hovered = hovered;
                }
                Task::none()
            }
            Message::AgentPnlCardLoaded(generation, result) => {
                self.handle_agent_pnl_card_loaded(generation, result.into_result())
            }
            Message::AgentPnlCardRemove => {
                if !self.agent.status.is_busy() {
                    self.agent.clear_pnl_card_attachment();
                }
                Task::none()
            }
            Message::AgentSubmit => self.submit_agent_prompt(),
            Message::AgentSnapshotPrepared(generation, request_id, result) => {
                self.handle_agent_snapshot_prepared(generation, request_id, result)
            }
            Message::AgentRuntimeEvent(event) => self.handle_agent_runtime_event(event),
            Message::AgentStreamTick => self.advance_agent_stream_presentation(),
            Message::AgentAbort => {
                self.agent.suppress_empty_response_retry = true;
                self.agent.workspace_actions_allowed = false;
                self.agent.finish_running_tools(true);
                self.agent.finish_reasoning();
                self.agent.flush_assistant_stream();
                agent_runtime::abort(self.agent.runtime_generation);
                self.agent.status_detail = Some("Stopping the current response…".to_string());
                Task::none()
            }
            Message::AgentCopyResponse(entry_index) => {
                let response = self
                    .agent
                    .entries
                    .get(entry_index)
                    .and_then(|entry| match entry {
                        AgentChatEntry::Message {
                            role: AgentChatRole::Assistant,
                            text,
                            ..
                        } => Some(text.clone()),
                        _ => None,
                    });
                response.map_or_else(Task::none, |response| {
                    self.update(Message::CopyToClipboard(response.into()))
                })
            }
            Message::AgentRegenerateResponse(entry_index) => {
                self.regenerate_agent_response(entry_index)
            }
            Message::AgentToggleToolTrace(entry_index) => {
                self.agent.toggle_tool_trace(entry_index);
                Task::none()
            }
            Message::AgentToggleReasoning(entry_index) => {
                self.agent.toggle_reasoning(entry_index);
                Task::none()
            }
            Message::AgentFollowUpSelected(prompt) => {
                if self.agent.status.is_busy() {
                    return Task::none();
                }
                self.agent.input = prompt.into_string();
                iced::widget::operation::focus(iced::widget::Id::new("kerosene-agent-input"))
            }
            Message::AgentNewChat => self.create_agent_session(),
            Message::AgentSelectSession(id) => self.select_agent_session(id),
            Message::AgentToggleSidebar => {
                self.agent.sidebar_collapsed = !self.agent.sidebar_collapsed;
                Task::none()
            }
            Message::AgentProviderChanged(provider) => self.change_agent_provider(provider),
            Message::AgentLocalServerDetected(generation, result) => {
                self.handle_local_server_detected(generation, result)
            }
            Message::AgentToggleModelPicker => {
                if self.agent.model_picker_open {
                    self.agent.model_picker_open = false;
                    self.agent.model_search.clear();
                    Task::none()
                } else {
                    self.agent.model_picker_open = true;
                    match self.assistant_provider {
                        AssistantProvider::OpenRouter
                            if self.agent.model_catalog.is_empty()
                                && !self.agent.model_catalog_loading =>
                        {
                            self.load_agent_model_catalog()
                        }
                        AssistantProvider::LlamaCpp
                            if self.agent.local_server.is_none()
                                && !self.agent.local_detection_loading =>
                        {
                            self.detect_local_llama_cpp()
                        }
                        _ => Task::none(),
                    }
                }
            }
            Message::AgentModelSearchChanged(search) => {
                self.agent.model_search = search.chars().take(160).collect();
                Task::none()
            }
            Message::AgentRefreshModels => match self.assistant_provider {
                AssistantProvider::OpenRouter => self.load_agent_model_catalog(),
                AssistantProvider::LlamaCpp => self.detect_local_llama_cpp(),
            },
            Message::AgentModelCatalogLoaded(generation, result) => {
                if !self.openrouter_key_generation_is_current(generation) {
                    return Task::none();
                }
                self.agent.model_catalog_loading = false;
                match result {
                    Ok(models) => {
                        self.agent.model_catalog = models;
                        self.agent.model_catalog_error = None;
                        if self.agent.pnl_card_attachment.is_some() {
                            let model = self.assistant_model_for_task();
                            if model
                                .as_deref()
                                .and_then(|model| self.assistant_model_supports_images(model))
                                == Some(true)
                            {
                                self.agent.status_detail = None;
                            }
                        }
                    }
                    Err(error) => {
                        self.agent.model_catalog_error = Some(redact_sensitive_response_text(
                            &format!("Could not load OpenRouter models: {error}"),
                        ));
                    }
                }
                Task::none()
            }
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
        if self.config_clear_requested || self.config_cleared_this_session {
            self.agent.status_detail = Some(
                "Assistant sessions are unavailable until restart while config persistence is paused."
                    .to_string(),
            );
            return Task::none();
        }

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
            let detection = self.detect_local_llama_cpp();
            return Task::batch([focus, journal, detection]);
        }

        if !self.assistant_configured() {
            self.agent.status = AgentStatus::Error;
            self.agent.status_detail = Some(match self.assistant_provider {
                AssistantProvider::OpenRouter => {
                    "Add an OpenRouter API key or choose a detected local llama.cpp server."
                        .to_string()
                }
                AssistantProvider::LlamaCpp => {
                    "Detecting a compatible local llama.cpp server…".to_string()
                }
            });
        } else if self.agent.status == AgentStatus::Error {
            self.agent.status = AgentStatus::Stopped;
            self.agent.status_detail = None;
        }

        let settings = window::Settings {
            size: Size::new(940.0, 720.0),
            min_size: Some(Size::new(720.0, 480.0)),
            ..crate::window_chrome::settings(
                self.custom_window_chrome_active,
                self.window_background_blur_enabled,
            )
        };
        let (id, task) = window::open(settings);
        self.agent.window_id = Some(id);
        let journal = if self.connected_address.is_some() {
            self.load_journal_for_active_account(false)
        } else {
            Task::none()
        };
        let detection = self.detect_local_llama_cpp();
        Task::batch([task.map(Message::WindowOpened), journal, detection])
    }

    fn snap_agent_chat_to_latest(&self) -> Task<Message> {
        iced::widget::operation::snap_to(
            iced::widget::Id::new("kerosene-agent-chat"),
            iced::widget::scrollable::RelativeOffset::END,
        )
    }
}

#[cfg(test)]
mod tests;
