use crate::api::fetch_hype_etfs;
use crate::app_state::TradingTerminal;
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

#[cfg(test)]
mod tests {
    use super::*;

    use iced::widget::pane_grid;

    #[test]
    fn boot_refresh_only_starts_when_hype_etf_pane_is_open() {
        let (mut terminal, _) = TradingTerminal::boot();
        let (panes, pane) = pane_grid::State::new(PaneKind::Chart(0));
        terminal.panes = panes;
        terminal.hype_etfs.loading = false;

        let _task = terminal.request_hype_etfs_boot_refresh();
        assert!(!terminal.hype_etfs.loading);

        terminal
            .panes
            .split(pane_grid::Axis::Vertical, pane, PaneKind::HypeEtfs)
            .expect("split should create HYPE ETF pane");

        let _task = terminal.request_hype_etfs_boot_refresh();
        assert!(terminal.hype_etfs.loading);
    }
}
