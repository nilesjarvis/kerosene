mod input;
mod models;
mod turns;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_assistant(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AssistantModelsRefresh => self.refresh_assistant_models(),
            Message::AssistantModelsLoaded(result) => self.apply_assistant_models_loaded(result),
            Message::AssistantModelSelected(model) => self.select_assistant_model(model),
            Message::AssistantInputChanged(value) => self.update_assistant_input(value),
            Message::AssistantInsertTicker(symbol_key) => self.insert_assistant_ticker(symbol_key),
            Message::AssistantSend => self.send_assistant_prompt(),
            Message::AssistantPlanLoaded(result) => self.apply_assistant_plan_loaded(result),
            Message::AssistantExecuteLoaded(result) => self.apply_assistant_execute_loaded(result),
            Message::AssistantCopyText(content) => self.copy_assistant_text(content),
            Message::AssistantToggleAccountContext(enabled) => {
                self.set_assistant_account_context(enabled)
            }
            Message::AssistantToggleCodeExecution(enabled) => {
                self.set_assistant_code_execution(enabled)
            }
            Message::AssistantClearChat => self.clear_assistant_chat(),
            _ => Task::none(),
        }
    }
}
