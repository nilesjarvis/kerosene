use crate::api::fetch_hype_unstaking_queue;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Task;
use std::time::{Duration, Instant};

const HYPE_UNSTAKING_QUEUE_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);

// ---------------------------------------------------------------------------
// HYPE Unstaking Queue Updates
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn request_hype_unstaking_queue_boot_refresh(&mut self) -> Task<Message> {
        if self.pane_is_open(|kind| matches!(kind, PaneKind::HypeUnstakingQueue)) {
            self.request_hype_unstaking_queue_refresh(false)
        } else {
            Task::none()
        }
    }

    pub(crate) fn update_hype_unstaking_queue_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshHypeUnstakingQueue => self.request_hype_unstaking_queue_refresh(true),
            Message::HypeUnstakingQueueRefreshTick => {
                self.request_hype_unstaking_queue_refresh(false)
            }
            Message::HypeUnstakingWindowChanged(filter) => {
                self.hype_unstaking_queue.window_filter = filter;
                Task::none()
            }
            Message::HypeUnstakingAmountFilterChanged(filter) => {
                self.hype_unstaking_queue.amount_filter = filter;
                Task::none()
            }
            Message::HypeUnstakingSortChanged(field) => {
                self.hype_unstaking_queue.apply_sort_change(field);
                Task::none()
            }
            Message::ToggleHypeUnstakingMineOnly => {
                self.hype_unstaking_queue.mine_only = !self.hype_unstaking_queue.mine_only;
                Task::none()
            }
            Message::ClearHypeUnstakingFilters => {
                self.hype_unstaking_queue.clear_filters();
                Task::none()
            }
            Message::HypeUnstakingQueueLoaded(result) => {
                self.hype_unstaking_queue.loading = false;
                self.hype_unstaking_queue.last_fetch = Some(Instant::now());
                match *result {
                    Ok(data) => {
                        self.hype_unstaking_queue.data = Some(data);
                        self.hype_unstaking_queue.error = None;
                    }
                    Err(error) => {
                        self.hype_unstaking_queue.error = Some(error);
                    }
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_hype_unstaking_queue_refresh(&mut self, force: bool) -> Task<Message> {
        if self.hype_unstaking_queue.loading {
            return Task::none();
        }

        if !force
            && self
                .hype_unstaking_queue
                .last_fetch
                .is_some_and(|last_fetch| {
                    last_fetch.elapsed() < HYPE_UNSTAKING_QUEUE_REFRESH_INTERVAL
                })
        {
            return Task::none();
        }

        self.hype_unstaking_queue.loading = true;
        if force || self.hype_unstaking_queue.data.is_none() {
            self.hype_unstaking_queue.error = None;
        }

        Task::perform(fetch_hype_unstaking_queue(), |result| {
            Message::HypeUnstakingQueueLoaded(Box::new(result))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use iced::widget::pane_grid;

    #[test]
    fn boot_refresh_is_noop_when_pane_is_closed() {
        let (mut terminal, _) = TradingTerminal::boot();
        let (panes, _) = pane_grid::State::new(PaneKind::Chart(0));
        terminal.panes = panes;
        terminal.hype_unstaking_queue.loading = false;

        let _ = terminal.request_hype_unstaking_queue_boot_refresh();
        assert!(
            !terminal.hype_unstaking_queue.loading,
            "should not start refresh when pane is closed"
        );
    }

    #[test]
    fn boot_refresh_starts_refresh_when_pane_is_open() {
        let (mut terminal, _) = TradingTerminal::boot();
        let (panes, pane) = pane_grid::State::new(PaneKind::Chart(0));
        terminal.panes = panes;
        terminal.hype_unstaking_queue.loading = false;

        terminal
            .panes
            .split(
                pane_grid::Axis::Vertical,
                pane,
                PaneKind::HypeUnstakingQueue,
            )
            .expect("split should create HYPE unstaking queue pane");

        let _ = terminal.request_hype_unstaking_queue_boot_refresh();
        assert!(
            terminal.hype_unstaking_queue.loading,
            "should start refresh when pane is open"
        );
    }
}
