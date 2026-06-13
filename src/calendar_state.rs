use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use crate::api;
use iced::Task;

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
        self.calendar_request_id = self.calendar_request_id.wrapping_add(1);
        let request_id = self.calendar_request_id;
        if force {
            self.calendar_error = None;
            self.calendar_retry_attempts = 0;
            self.calendar_next_retry = None;
        }

        Task::perform(api::fetch_economic_calendar(), move |result| {
            Message::CalendarLoaded(request_id, result)
        })
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
