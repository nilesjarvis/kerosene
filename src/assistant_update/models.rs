use crate::app_state::TradingTerminal;
use crate::assistant;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(super) fn refresh_assistant_models(&mut self) -> Task<Message> {
        let url = self.assistant.ollama_url.clone();
        self.assistant.loading = true;
        self.assistant.last_error = None;
        Task::perform(
            async move { assistant::list_models(&url).await },
            Message::AssistantModelsLoaded,
        )
    }

    pub(super) fn apply_assistant_models_loaded(
        &mut self,
        result: Result<Vec<String>, String>,
    ) -> Task<Message> {
        self.assistant.loading = false;
        match result {
            Ok(models) => {
                self.assistant.models = models;
                if self.assistant.selected_model.is_none()
                    && let Some(first) = self.assistant.models.first()
                {
                    self.assistant.selected_model = Some(first.clone());
                } else if let Some(selected) = &self.assistant.selected_model
                    && !self.assistant.models.iter().any(|model| model == selected)
                {
                    self.assistant.selected_model = self.assistant.models.first().cloned();
                }
                self.assistant.last_error = None;
                self.persist_config();
            }
            Err(error) => {
                self.assistant.last_error = Some(error);
            }
        }
        Task::none()
    }

    pub(super) fn select_assistant_model(&mut self, model: String) -> Task<Message> {
        self.assistant.selected_model = Some(model);
        self.persist_config();
        Task::none()
    }
}
