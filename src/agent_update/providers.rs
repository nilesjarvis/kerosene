use crate::agent_state::AgentStatus;
use crate::app_state::TradingTerminal;
use crate::config::AssistantProvider;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::Task;

// ---------------------------------------------------------------------------
// Assistant Model Providers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn load_agent_model_catalog(&mut self) -> Task<Message> {
        if !self.openrouter_configured() {
            self.agent.model_catalog_loading = false;
            self.agent.model_catalog_error = Some(
                "Add an OpenRouter API key in Settings → Integrations to load models.".to_string(),
            );
            return Task::none();
        }

        self.agent.model_catalog_loading = true;
        self.agent.model_catalog_error = None;
        let generation = self.openrouter_key_generation;
        Task::perform(
            crate::openrouter_api::fetch_tool_models(self.openrouter_api_key_for_task()),
            move |result| Message::AgentModelCatalogLoaded(generation, result),
        )
    }

    pub(super) fn detect_local_llama_cpp(&mut self) -> Task<Message> {
        if self.agent.local_detection_loading {
            return Task::none();
        }
        let generation = self.agent.begin_local_detection();
        Task::perform(crate::llama_cpp::detect_server(), move |result| {
            Message::AgentLocalServerDetected(generation, result)
        })
    }

    pub(super) fn handle_local_server_detected(
        &mut self,
        generation: u64,
        result: Result<Option<crate::llama_cpp::LlamaCppServer>, String>,
    ) -> Task<Message> {
        if generation != self.agent.local_detection_generation {
            return Task::none();
        }
        self.agent.local_detection_loading = false;

        let previous = self.agent.local_server.take();
        match result {
            Ok(server) => {
                self.agent.local_detection_error = None;
                self.agent.local_server = server;
            }
            Err(error) => {
                self.agent.local_server = None;
                self.agent.local_detection_error = Some(redact_sensitive_response_text(&error));
            }
        }

        if self.assistant_provider == AssistantProvider::LlamaCpp {
            if previous != self.agent.local_server && self.agent.runtime_connected {
                self.invalidate_agent_runtime();
            }
            match self.agent.local_server.as_ref() {
                Some(server) if server.supports_tools && server.primary_model().is_some() => {
                    if !self.agent.status.is_busy() {
                        self.agent.status = AgentStatus::Stopped;
                        self.agent.status_detail = None;
                    }
                }
                Some(_) => {
                    self.agent.status = AgentStatus::Error;
                    self.agent.status_detail = Some(
                        "A local llama.cpp server was detected, but its chat template does not advertise tool calling required by the Assistant."
                            .to_string(),
                    );
                }
                None => {
                    self.agent.status = AgentStatus::Error;
                    self.agent.status_detail = Some(
                        self.agent.local_detection_error.clone().unwrap_or_else(|| {
                            "No compatible llama.cpp server was detected on this machine. Start llama-server with its OpenAI-compatible API enabled, then refresh."
                                .to_string()
                        }),
                    );
                }
            }
        } else if !self.openrouter_configured()
            && self
                .agent
                .local_server
                .as_ref()
                .is_some_and(|server| server.supports_tools)
        {
            self.agent.model_picker_open = true;
            self.agent.status_detail = Some(
                "A compatible local llama.cpp server was detected. Choose Local llama.cpp below to use it without an OpenRouter key."
                    .to_string(),
            );
        }
        Task::none()
    }

    pub(super) fn change_agent_provider(&mut self, provider: AssistantProvider) -> Task<Message> {
        if self.agent.status.is_busy() || self.assistant_provider == provider {
            return Task::none();
        }

        self.invalidate_agent_runtime();
        self.assistant_provider = provider;
        self.agent.model_picker_open = false;
        self.agent.model_search.clear();
        self.persist_config();

        let model = self.assistant_model_for_task();
        if self.agent.pnl_card_attachment.is_some()
            && model
                .as_deref()
                .and_then(|model| self.assistant_model_supports_images(model))
                != Some(true)
        {
            self.agent.status_detail = Some(
                "The selected provider does not expose a vision-capable model for this P&L card."
                    .to_string(),
            );
        } else if self.assistant_configured() {
            self.agent.status = AgentStatus::Stopped;
            self.agent.status_detail = None;
        } else {
            self.agent.status = AgentStatus::Error;
            self.agent.status_detail = Some(match provider {
                AssistantProvider::OpenRouter => {
                    "Add an OpenRouter API key in Settings → Integrations before sending."
                        .to_string()
                }
                AssistantProvider::LlamaCpp => {
                    "Detecting a compatible local llama.cpp server…".to_string()
                }
            });
        }

        if provider == AssistantProvider::LlamaCpp && self.agent.local_server.is_none() {
            self.detect_local_llama_cpp()
        } else {
            Task::none()
        }
    }
}
