use crate::api::fetch_hype_etfs;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;
use std::time::{Duration, Instant};

const HYPE_ETF_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);

// ---------------------------------------------------------------------------
// HYPE ETF Updates
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn update_hype_etfs_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshHypeEtfs => self.request_hype_etfs_refresh(true),
            Message::HypeEtfsRefreshTick => self.request_hype_etfs_refresh(false),
            Message::HypeEtfsViewChanged(view) => {
                self.hype_etfs.view = view;
                Task::none()
            }
            Message::HypeEtfsLoaded(result) => {
                self.hype_etfs.loading = false;
                self.hype_etfs.last_fetch = Some(Instant::now());
                match *result {
                    Ok(data) => {
                        self.hype_etfs.data = Some(data);
                        self.hype_etfs.error = None;
                    }
                    Err(error) => {
                        self.hype_etfs.error = Some(error);
                    }
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_hype_etfs_refresh(&mut self, force: bool) -> Task<Message> {
        if self.hype_etfs.loading {
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
        Task::perform(fetch_hype_etfs(), |result| {
            Message::HypeEtfsLoaded(Box::new(result))
        })
    }
}
