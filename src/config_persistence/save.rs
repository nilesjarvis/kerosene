use crate::app_state::TradingTerminal;
use crate::config::{self, save_config};
use crate::message::Message;
use iced::Task;
use std::time::{Duration, Instant};

mod lifecycle;
mod snapshot;

#[cfg(test)]
use lifecycle::config_save_is_due;
use lifecycle::{
    ConfigSaveCompletionAction, config_save_completion_action, config_save_should_start,
};

const CONFIG_SAVE_DEBOUNCE: Duration = Duration::from_millis(750);

async fn save_config_off_thread(config: config::KeroseneConfig) -> Result<(), String> {
    tokio::task::spawn_blocking(move || save_config(&config))
        .await
        .map_err(|e| format!("config save task failed: {e}"))?
}

#[cfg(test)]
mod tests;

impl TradingTerminal {
    /// Request a config save after the debounce window.
    pub(crate) fn persist_config(&mut self) {
        if self.config_cleared_this_session {
            return;
        }
        self.config_save_due_at = Some(Instant::now() + CONFIG_SAVE_DEBOUNCE);
    }

    pub(crate) fn flush_config_save_if_due(&mut self, now: Instant) -> Task<Message> {
        if !config_save_should_start(self.config_save_due_at, self.config_save_in_flight, now) {
            return Task::none();
        }
        self.config_save_due_at = None;
        self.start_config_save()
    }

    pub(crate) fn flush_pending_config_save_and_exit(&mut self) -> Task<Message> {
        self.config_save_exit_requested = true;
        if self.config_cleared_this_session {
            self.config_save_due_at = None;
            self.config_save_in_flight = false;
            self.config_save_exit_requested = false;
            return iced::exit();
        }

        if self.config_save_in_flight {
            return Task::none();
        }

        if self.config_save_due_at.take().is_some() {
            return self.start_config_save();
        }

        self.config_save_exit_requested = false;
        iced::exit()
    }

    pub(crate) fn handle_config_save_result(
        &mut self,
        result: Result<(), String>,
    ) -> Task<Message> {
        self.config_save_in_flight = false;
        let save_succeeded = result.is_ok();
        self.record_config_save_result(result);

        match config_save_completion_action(
            self.config_save_exit_requested,
            self.config_save_due_at.is_some(),
            save_succeeded,
        ) {
            ConfigSaveCompletionAction::SavePending => {
                self.config_save_due_at = None;
                self.start_config_save()
            }
            ConfigSaveCompletionAction::Exit => {
                self.config_save_exit_requested = false;
                iced::exit()
            }
            ConfigSaveCompletionAction::BlockExitOnError => {
                // Clear the exit-requested flag but keep a save due now. A
                // subsequent close re-runs the final save instead of silently
                // discarding the user's latest layout/preferences after the
                // first failed write.
                self.config_save_exit_requested = false;
                self.config_save_due_at = Some(Instant::now());
                self.push_toast(
                    "Config save failed; close again to retry or keep app open.".to_string(),
                    true,
                );
                Task::none()
            }
            ConfigSaveCompletionAction::None => Task::none(),
        }
    }

    fn start_config_save(&mut self) -> Task<Message> {
        if self.config_cleared_this_session {
            self.config_save_due_at = None;
            return Task::none();
        }
        if self.config_save_in_flight {
            return Task::none();
        }

        let config = self.config_snapshot();
        self.config_save_in_flight = true;
        Task::perform(save_config_off_thread(config), Message::ConfigSaved)
    }

    fn record_config_save_result(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                if self
                    .secret_store_status
                    .as_ref()
                    .is_some_and(|(status, _)| status.starts_with("Config save failed"))
                {
                    self.secret_store_status = Some(("Config saved".to_string(), false));
                }
            }
            Err(e) => {
                let message = format!("Config save failed: {e}");
                eprintln!("{message}");
                self.secret_store_status = Some((message, true));
            }
        }
    }
}
