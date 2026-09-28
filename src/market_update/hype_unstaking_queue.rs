use crate::api::fetch_hype_unstaking_queue;
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
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
            Message::HypeUnstakingQueueLoaded(request_id, result) => {
                if !self.hype_unstaking_queue.loading
                    || request_id != self.hype_unstaking_queue.refresh_request_id
                {
                    return Task::none();
                }

                self.hype_unstaking_queue.loading = false;
                match *result {
                    Ok(mut data) => {
                        data.retain_upcoming_events(Self::now_ms());
                        self.hype_unstaking_queue.last_fetch = Some(Instant::now());
                        self.hype_unstaking_queue.data = Some(data);
                        self.hype_unstaking_queue.error = None;
                    }
                    Err(error) => {
                        self.hype_unstaking_queue.error =
                            Some(redact_sensitive_response_text(&error));
                    }
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_hype_unstaking_queue_refresh(&mut self, force: bool) -> Task<Message> {
        if self.hype_unstaking_queue.loading
            || (!force && !self.pane_is_open(|kind| matches!(kind, PaneKind::HypeUnstakingQueue)))
        {
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

        self.hype_unstaking_queue.refresh_request_id =
            self.hype_unstaking_queue.refresh_request_id.wrapping_add(1);
        let request_id = self.hype_unstaking_queue.refresh_request_id;
        Task::perform(fetch_hype_unstaking_queue(), move |result| {
            Message::HypeUnstakingQueueLoaded(request_id, Box::new(result))
        })
    }
}

#[cfg(test)]
mod tests;
