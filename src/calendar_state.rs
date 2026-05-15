use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use crate::api;
use iced::Task;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalendarImpactFilter {
    MediumHigh,
    High,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalendarWindowFilter {
    Upcoming,
    Today,
    Week,
}

impl TradingTerminal {
    pub(crate) fn is_calendar_open(&self) -> bool {
        self.panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::Calendar))
    }

    pub(crate) fn request_calendar_refresh(&mut self, force: bool) -> Task<Message> {
        if self.calendar_loading || (!force && !self.is_calendar_open()) {
            return Task::none();
        }

        self.calendar_loading = true;
        if force {
            self.calendar_error = None;
            self.calendar_retry_attempts = 0;
            self.calendar_next_retry = None;
        }

        Task::perform(api::fetch_economic_calendar(), Message::CalendarLoaded)
    }

    pub(crate) fn calendar_refresh_due(&self, now: Instant) -> bool {
        if self.calendar_loading || !self.is_calendar_open() {
            return false;
        }

        if let Some(retry_at) = self.calendar_next_retry {
            return now >= retry_at;
        }

        self.calendar_last_fetch
            .is_none_or(|last_fetch| now.duration_since(last_fetch).as_secs() >= 15 * 60)
    }

    pub(crate) fn calendar_retry_delay_secs(attempts: u8) -> u64 {
        match attempts {
            0 => 5,
            1 => 15,
            2 => 60,
            _ => 5 * 60,
        }
    }
}
