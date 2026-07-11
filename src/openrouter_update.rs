use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::openrouter_api::{self, OpenRouterKeyStatus};
use iced::Task;
use zeroize::{Zeroize, Zeroizing};

/// Runtime-only identity for one key validation task.
///
/// Key generation protects credential replacement; the independent request ID
/// distinguishes repeated checks of the same configured key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpenRouterKeyCheckRequest {
    request_id: u64,
    key_generation: u64,
}

impl OpenRouterKeyCheckRequest {
    pub(crate) fn new(request_id: u64, key_generation: u64) -> Self {
        Self {
            request_id,
            key_generation,
        }
    }

    fn key_generation(self) -> u64 {
        self.key_generation
    }

    #[cfg(test)]
    pub(crate) fn request_id(self) -> u64 {
        self.request_id
    }
}

impl TradingTerminal {
    fn next_openrouter_key_check_request(&mut self) -> OpenRouterKeyCheckRequest {
        self.openrouter_key_check_next_request_id =
            self.openrouter_key_check_next_request_id.wrapping_add(1);
        OpenRouterKeyCheckRequest::new(
            self.openrouter_key_check_next_request_id,
            self.openrouter_key_generation,
        )
    }

    pub(crate) fn update_openrouter(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenRouterKeyInputChanged(value) => {
                self.openrouter_key_input.zeroize();
                self.openrouter_key_input = value.into_zeroizing().into();
            }
            Message::SaveOpenRouterKey => {
                let previous_key = Zeroizing::new(self.openrouter_api_key.trim().to_string());
                let next_key = Zeroizing::new(self.openrouter_key_input.trim().to_string());
                if !self.persist_openrouter_secret_from_key(next_key.as_str()) {
                    return Task::none();
                }

                self.openrouter_api_key.zeroize();
                self.openrouter_api_key = next_key.as_str().to_string().into();
                let openrouter_key_changed = previous_key.as_str() != next_key.as_str();
                if openrouter_key_changed {
                    self.bump_openrouter_key_generation();
                }
                self.persist_config();
                if self.openrouter_api_key.trim().is_empty() {
                    self.openrouter_key_status = None;
                    self.openrouter_key_check_request = None;
                    return Task::none();
                }

                self.openrouter_key_status = Some(("Checking key...".to_string(), false));
                let request = self.next_openrouter_key_check_request();
                self.openrouter_key_check_request = Some(request);
                return Task::perform(
                    openrouter_api::fetch_key_status(self.openrouter_api_key_for_task()),
                    move |result| Message::OpenRouterKeyChecked(request, result.into()),
                );
            }
            Message::OpenRouterKeyChecked(request, result) => {
                if self.openrouter_key_check_request != Some(request) {
                    return Task::none();
                }
                self.openrouter_key_check_request = None;
                if !self.openrouter_key_generation_is_current(request.key_generation())
                    || self.openrouter_api_key.trim().is_empty()
                {
                    return Task::none();
                }
                self.openrouter_key_status = Some(match result.into_result() {
                    Ok(status) => (openrouter_key_status_message(&status), false),
                    Err(error) => (redact_sensitive_response_text(&error), true),
                });
            }
            Message::OpenRouterModelChanged(value) => {
                self.openrouter_model = value;
                self.persist_config();
            }
            _ => {}
        }

        Task::none()
    }
}

fn openrouter_key_status_message(status: &OpenRouterKeyStatus) -> String {
    let mut message = format!("Key valid — ${:.2} used", status.usage_usd);
    match (status.limit_remaining_usd, status.limit_usd) {
        (Some(remaining), Some(limit)) => {
            message.push_str(&format!(", ${remaining:.2} of ${limit:.2} limit left"));
        }
        (Some(remaining), None) => {
            message.push_str(&format!(", ${remaining:.2} left"));
        }
        _ => {}
    }
    if status.is_free_tier {
        message.push_str(" (free tier)");
    }
    message
}

#[cfg(test)]
mod tests;
