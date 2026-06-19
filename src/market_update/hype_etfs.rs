use crate::api::fetch_hype_etfs;
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Task;
use std::time::{Duration, Instant};

const HYPE_ETF_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);

// ---------------------------------------------------------------------------
// HYPE ETF Updates
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn request_hype_etfs_boot_refresh(&mut self) -> Task<Message> {
        if self.pane_is_open(|kind| matches!(kind, PaneKind::HypeEtfs)) {
            self.request_hype_etfs_refresh(false)
        } else {
            Task::none()
        }
    }

    pub(crate) fn update_hype_etfs_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshHypeEtfs => self.request_hype_etfs_refresh(true),
            Message::HypeEtfsRefreshTick => self.request_hype_etfs_refresh(false),
            Message::HypeEtfsViewChanged(view) => {
                self.hype_etfs.view = view;
                Task::none()
            }
            Message::HypeEtfsLoaded(request_id, result) => {
                if !self.hype_etfs.loading || request_id != self.hype_etfs.refresh_request_id {
                    return Task::none();
                }

                self.hype_etfs.loading = false;
                match *result {
                    Ok(data) => {
                        self.hype_etfs.last_fetch = Some(Instant::now());
                        self.hype_etfs.data = Some(data);
                        self.hype_etfs.error = None;
                    }
                    Err(error) => {
                        self.hype_etfs.error = Some(redact_sensitive_response_text(&error));
                    }
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_hype_etfs_refresh(&mut self, force: bool) -> Task<Message> {
        if self.hype_etfs.loading
            || (!force && !self.pane_is_open(|kind| matches!(kind, PaneKind::HypeEtfs)))
        {
            return Task::none();
        }

        if !force
            && self
                .hype_etfs
                .last_fetch
                .is_some_and(|last_fetch| last_fetch.elapsed() < HYPE_ETF_REFRESH_INTERVAL)
        {
            return Task::none();
        }

        self.hype_etfs.loading = true;
        self.hype_etfs.error = None;
        self.hype_etfs.refresh_request_id = self.hype_etfs.refresh_request_id.wrapping_add(1);
        let request_id = self.hype_etfs.refresh_request_id;
        Task::perform(fetch_hype_etfs(), move |result| {
            Message::HypeEtfsLoaded(request_id, Box::new(result))
        })
    }
}

#[cfg(test)]
mod tests;
