use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::openrouter_api::{self, OpenRouterKeyStatus};
use iced::Task;
use zeroize::{Zeroize, Zeroizing};

impl TradingTerminal {
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
                    return Task::none();
                }

                self.openrouter_key_status = Some(("Checking key...".to_string(), false));
                let generation = self.openrouter_key_generation;
                return Task::perform(
                    openrouter_api::fetch_key_status(self.openrouter_api_key_for_task()),
                    move |result| Message::OpenRouterKeyChecked(generation, result),
                );
            }
            Message::OpenRouterKeyChecked(generation, result) => {
                if !self.openrouter_key_generation_is_current(generation)
                    || self.openrouter_api_key.trim().is_empty()
                {
                    return Task::none();
                }
                self.openrouter_key_status = Some(match result {
                    Ok(status) => (openrouter_key_status_message(&status), false),
                    Err(error) => (error, true),
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
