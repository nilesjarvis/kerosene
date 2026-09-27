use crate::app_state::TradingTerminal;
use crate::market_state::listings::LISTINGS_REFRESH_SECS;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use std::time::{Duration, Instant};

impl TradingTerminal {
    pub(crate) fn update_listings_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ListingsTick => {
                self.listings.now_ms = crate::app_time::now_ms();
                self.request_listings_refresh(false)
            }
            Message::RefreshListings => self.request_listings_refresh(true),
            Message::ListingsFilterChanged(filter) => {
                self.listings.filter = filter;
                Task::none()
            }
            Message::ListingsLoaded(request_id, snapshot) => {
                if !self.listings.loading || request_id != self.listings.request_id {
                    return Task::none();
                }
                self.listings.loading = false;
                self.listings.now_ms = crate::app_time::now_ms();
                let added = self.listings.apply(*snapshot, self.listings.now_ms);
                let save = self.save_listings_history();
                if added {
                    Task::batch([save, self.request_exchange_symbols_refresh()])
                } else {
                    save
                }
            }
            Message::ListingsSaved(result) => {
                if !self.listings.saving {
                    return Task::none();
                }
                self.listings.saving = false;
                self.listings.storage_error = result.is_err();
                self.listings.dirty |= result.is_err();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_listings_refresh(&mut self, force: bool) -> Task<Message> {
        if self.listings.loading
            || self.listings.saving
            || !self.pane_is_open(|kind| matches!(kind, PaneKind::NewListings))
        {
            return Task::none();
        }
        // Manual refreshes also have a short cooldown to avoid request storms.
        let cooldown = Duration::from_secs(if force { 5 } else { LISTINGS_REFRESH_SECS });
        if self
            .listings
            .last_attempt
            .is_some_and(|at| at.elapsed() < cooldown)
        {
            return Task::none();
        }
        self.listings.loading = true;
        self.listings.last_attempt = Some(Instant::now());
        self.listings.request_id = self.listings.request_id.wrapping_add(1);
        let request_id = self.listings.request_id;
        Task::perform(crate::api::fetch_listings_snapshot(), move |snapshot| {
            Message::ListingsLoaded(request_id, Box::new(snapshot))
        })
    }

    fn save_listings_history(&mut self) -> Task<Message> {
        if !self.listings.dirty || self.listings.saving {
            return Task::none();
        }
        self.listings.dirty = false;
        self.listings.saving = true;
        Task::perform(
            crate::config_persistence::listings::save(self.listings.history.clone()),
            Message::ListingsSaved,
        )
    }
}

#[cfg(test)]
mod tests;
