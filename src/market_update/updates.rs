use crate::{app_state::TradingTerminal, message::Message, pane_state::PaneKind};
use iced::Task;
use std::time::{Duration, Instant};

impl TradingTerminal {
    pub(crate) fn update_parameters_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UpdatesTick => {
                self.updates.now_ms = crate::app_time::now_ms();
                Task::batch([
                    self.request_updates_refresh(false),
                    self.save_updates_history(),
                ])
            }
            Message::RefreshUpdates => self.request_updates_refresh(true),
            Message::UpdatesFilterChanged(filter) => {
                self.updates.filter = filter;
                Task::none()
            }
            Message::UpdatesVenueChanged(venue) => {
                self.updates.venue = venue;
                Task::none()
            }
            Message::UpdatesSearchChanged(search) => {
                self.updates.search = search.chars().take(128).collect();
                Task::none()
            }
            Message::UpdatesFavouritesChanged(only) => {
                self.updates.favourites_only = only;
                Task::none()
            }
            Message::UpdatesToggleDetails(at, key) => {
                if self
                    .updates
                    .history
                    .events
                    .iter()
                    .any(|event| event.detected_at_ms == at && event.key == key)
                    && !self.updates.expanded.remove(&(at, key.clone()))
                {
                    self.updates.expanded.insert((at, key));
                }
                Task::none()
            }
            Message::UpdatesLoaded(request_id, snapshot) => {
                if !self.updates.loading || request_id != self.updates.request_id {
                    return Task::none();
                }
                self.updates.loading = false;
                self.updates.now_ms = crate::app_time::now_ms();
                self.updates.apply(*snapshot, self.updates.now_ms);
                self.save_updates_history()
            }
            Message::UpdatesSaved(result) => {
                if self.updates.saving {
                    self.updates.saving = false;
                    self.updates.storage_error = result.is_err();
                    self.updates.dirty |= result.is_err();
                    // Flush any observations received while this save was running.
                    if result.is_ok() {
                        return self.save_updates_history();
                    }
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_updates_refresh(&mut self, force: bool) -> Task<Message> {
        if self.updates.loading || !self.pane_is_open(|kind| matches!(kind, PaneKind::Updates)) {
            return Task::none();
        }
        let cooldown = Duration::from_secs(if force {
            5
        } else {
            self.updates.refresh_secs()
        });
        if self
            .updates
            .last_attempt
            .is_some_and(|at| at.elapsed() < cooldown)
        {
            return Task::none();
        }
        self.updates.loading = true;
        self.updates.last_attempt = Some(Instant::now());
        self.updates.request_id = self.updates.request_id.wrapping_add(1);
        let id = self.updates.request_id;
        Task::perform(crate::api::fetch_updates_snapshot(), move |snapshot| {
            Message::UpdatesLoaded(id, Box::new(snapshot))
        })
    }

    fn save_updates_history(&mut self) -> Task<Message> {
        if !self.updates.dirty
            || self.updates.saving
            || (self.updates.storage_error
                && self
                    .updates
                    .last_save_attempt
                    .is_some_and(|at| at.elapsed() < Duration::from_secs(30)))
        {
            return Task::none();
        }
        self.updates.dirty = false;
        self.updates.saving = true;
        self.updates.last_save_attempt = Some(Instant::now());
        Task::perform(
            crate::config_persistence::updates::save(self.updates.history.clone()),
            Message::UpdatesSaved,
        )
    }
}

#[cfg(test)]
mod tests;
