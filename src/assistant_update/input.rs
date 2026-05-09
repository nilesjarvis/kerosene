use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::{Task, clipboard};

impl TradingTerminal {
    pub(super) fn update_assistant_input(&mut self, value: String) -> Task<Message> {
        self.assistant.input = value;
        Task::none()
    }

    pub(super) fn insert_assistant_ticker(&mut self, symbol_key: String) -> Task<Message> {
        let (start, _) = Self::assistant_symbol_query(&self.assistant.input)
            .unwrap_or((self.assistant.input.len(), String::new()));
        let replacement = format!("${{{symbol_key}}}");
        let prefix = &self.assistant.input[..start];
        self.assistant.input = format!("{prefix}{replacement} ");
        Task::none()
    }

    pub(super) fn copy_assistant_text(&mut self, content: String) -> Task<Message> {
        self.push_toast("Copied message".to_string(), false);
        clipboard::write(content)
    }

    pub(super) fn set_assistant_account_context(&mut self, enabled: bool) -> Task<Message> {
        self.assistant.use_account_context = enabled;
        self.persist_config();
        Task::none()
    }

    pub(super) fn set_assistant_code_execution(&mut self, enabled: bool) -> Task<Message> {
        self.assistant.allow_code_execution = enabled;
        self.persist_config();
        Task::none()
    }

    pub(super) fn clear_assistant_chat(&mut self) -> Task<Message> {
        self.assistant.history.clear();
        self.assistant.last_error = None;
        Task::none()
    }
}
