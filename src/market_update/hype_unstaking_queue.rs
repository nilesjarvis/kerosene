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
                match result.into_result() {
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
            Message::HypeUnstakingQueueLoaded(request_id, result.into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hype_unstaking_state::{HypeUnstakingEvent, HypeUnstakingQueueData};

    use iced::widget::pane_grid;

    fn open_hype_unstaking_queue_pane(terminal: &mut TradingTerminal) {
        let (panes, pane) = pane_grid::State::new(PaneKind::Chart(0));
        terminal.panes = panes;
        terminal
            .panes
            .split(
                pane_grid::Axis::Vertical,
                pane,
                PaneKind::HypeUnstakingQueue,
            )
            .expect("split should create HYPE unstaking queue pane");
    }

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
        assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 1);
    }

    #[test]
    fn hype_unstaking_refresh_wraps_request_id_without_replacing_active_owner() {
        let (mut terminal, _) = TradingTerminal::boot();
        open_hype_unstaking_queue_pane(&mut terminal);
        terminal.hype_unstaking_queue.refresh_request_id = u64::MAX;

        let _task = terminal.request_hype_unstaking_queue_refresh(false);

        assert!(terminal.hype_unstaking_queue.loading);
        assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 0);

        let _forced_task = terminal.request_hype_unstaking_queue_refresh(true);

        assert!(terminal.hype_unstaking_queue.loading);
        assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 0);
    }

    #[test]
    fn stale_loaded_message_does_not_clear_active_refresh() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.hype_unstaking_queue.loading = true;
        terminal.hype_unstaking_queue.refresh_request_id = 2;
        terminal.hype_unstaking_queue.error = Some("current error".to_string());

        let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
            1,
            Ok(HypeUnstakingQueueData::default()).into(),
        ));

        assert!(terminal.hype_unstaking_queue.loading);
        assert!(terminal.hype_unstaking_queue.data.is_none());
        assert_eq!(
            terminal.hype_unstaking_queue.error.as_deref(),
            Some("current error")
        );
        assert!(terminal.hype_unstaking_queue.last_fetch.is_none());
    }

    #[test]
    fn stale_error_does_not_change_current_unstaking_cache_or_owner() {
        let (mut terminal, _) = TradingTerminal::boot();
        let last_fetch = Instant::now();
        let current = HypeUnstakingEvent {
            unlock_time_ms: TradingTerminal::now_ms().saturating_add(60_000),
            user: "current-wallet".to_string(),
            amount_wei: 321,
        };
        terminal.hype_unstaking_queue.loading = true;
        terminal.hype_unstaking_queue.refresh_request_id = 9;
        terminal.hype_unstaking_queue.data =
            Some(HypeUnstakingQueueData::new(vec![current.clone()]));
        terminal.hype_unstaking_queue.error = Some("current error".to_string());
        terminal.hype_unstaking_queue.last_fetch = Some(last_fetch);

        let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
            8,
            Err("stale error".to_string()).into(),
        ));

        assert!(terminal.hype_unstaking_queue.loading);
        assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 9);
        assert_eq!(
            terminal
                .hype_unstaking_queue
                .data
                .as_ref()
                .expect("current data")
                .events,
            vec![current]
        );
        assert_eq!(
            terminal.hype_unstaking_queue.error.as_deref(),
            Some("current error")
        );
        assert_eq!(terminal.hype_unstaking_queue.last_fetch, Some(last_fetch));
    }

    #[test]
    fn duplicate_loaded_message_after_completion_is_ignored() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.hype_unstaking_queue.loading = false;
        terminal.hype_unstaking_queue.refresh_request_id = 7;
        terminal.hype_unstaking_queue.data =
            Some(HypeUnstakingQueueData::new(vec![HypeUnstakingEvent {
                unlock_time_ms: 1_000,
                user: "0xaccepted".to_string(),
                amount_wei: 100,
            }]));

        let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
            7,
            Ok(HypeUnstakingQueueData::new(vec![HypeUnstakingEvent {
                unlock_time_ms: 2_000,
                user: "0xduplicate".to_string(),
                amount_wei: 200,
            }]))
            .into(),
        ));

        let data = terminal
            .hype_unstaking_queue
            .data
            .as_ref()
            .expect("accepted data should remain cached");
        assert_eq!(data.events.len(), 1);
        assert_eq!(data.events[0].user, "0xaccepted");
    }

    #[test]
    fn loaded_refresh_prunes_unlocked_rows_before_caching() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.hype_unstaking_queue.loading = true;
        terminal.hype_unstaking_queue.refresh_request_id = 1;
        let now_ms = TradingTerminal::now_ms();

        let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
            1,
            Ok(HypeUnstakingQueueData::new(vec![
                HypeUnstakingEvent {
                    unlock_time_ms: now_ms.saturating_sub(1),
                    user: "0xpast".to_string(),
                    amount_wei: 100,
                },
                HypeUnstakingEvent {
                    unlock_time_ms: now_ms.saturating_add(60_000),
                    user: "0xfuture".to_string(),
                    amount_wei: 200,
                },
            ]))
            .into(),
        ));

        let data = terminal
            .hype_unstaking_queue
            .data
            .as_ref()
            .expect("successful refresh should cache data");
        assert_eq!(data.events.len(), 1);
        assert_eq!(data.events[0].user, "0xfuture");
        assert_eq!(data.events[0].amount_wei, 200);
    }

    #[test]
    fn failed_refresh_does_not_mark_hype_unstaking_queue_fresh() {
        let (mut terminal, _) = TradingTerminal::boot();
        open_hype_unstaking_queue_pane(&mut terminal);
        terminal.hype_unstaking_queue.loading = true;
        terminal.hype_unstaking_queue.refresh_request_id = 1;

        let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
            1,
            Err("network down".to_string()).into(),
        ));

        assert!(!terminal.hype_unstaking_queue.loading);
        assert_eq!(
            terminal.hype_unstaking_queue.error.as_deref(),
            Some("network down")
        );
        assert!(terminal.hype_unstaking_queue.last_fetch.is_none());

        let _task = terminal.request_hype_unstaking_queue_refresh(false);

        assert!(terminal.hype_unstaking_queue.loading);
        assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 2);
    }

    #[test]
    fn hype_unstaking_queue_error_redacts_state_error() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.hype_unstaking_queue.loading = true;
        terminal.hype_unstaking_queue.refresh_request_id = 1;

        let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
            1,
            Err("unstaking fetch failed: auth_token=unstaking-secret".to_string()).into(),
        ));

        let error = terminal
            .hype_unstaking_queue
            .error
            .as_deref()
            .expect("state error");
        assert!(error.contains("auth_token=<redacted>"));
        assert!(!error.contains("unstaking-secret"));
    }

    #[test]
    fn hype_unstaking_queue_tick_refresh_is_ignored_when_pane_is_closed() {
        let (mut terminal, _) = TradingTerminal::boot();
        let (panes, _) = pane_grid::State::new(PaneKind::Chart(0));
        terminal.panes = panes;

        let _task =
            terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueRefreshTick);

        assert!(!terminal.hype_unstaking_queue.loading);
        assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 0);
    }
}
