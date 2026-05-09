use crate::app_state::TradingTerminal;
use crate::message::Message;
use chrono::{DateTime, Local, Utc};
use iced::Task;
use std::time::{Duration, Instant};

impl TradingTerminal {
    pub(crate) fn update_calendar(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshCalendar => {
                return self.request_calendar_refresh(true);
            }
            Message::CalendarLoaded(result) => {
                let now = Instant::now();
                self.calendar_loading = false;
                match result {
                    Ok(events) => {
                        self.calendar_events = events;
                        self.calendar_error = None;
                        self.calendar_last_fetch = Some(now);
                        self.calendar_retry_attempts = 0;
                        self.calendar_next_retry = None;

                        let now = Utc::now();
                        let mut offset_y: f32 = 0.0;
                        let mut current_day = String::new();
                        let row_height: f32 = 40.0;
                        let spacing: f32 = 8.0;
                        let header_height: f32 = 44.0;

                        for event in self.calendar_events.iter() {
                            if let Ok(dt) = DateTime::parse_from_rfc3339(&event.date) {
                                let local_dt = dt.with_timezone(&Local);
                                let day_str = local_dt.format("%A, %b %e").to_string();

                                if day_str != current_day {
                                    current_day = day_str;
                                    offset_y += header_height;
                                }

                                if dt.with_timezone(&Utc) > now {
                                    break;
                                }

                                offset_y += row_height + spacing + 1.0;
                            }
                        }

                        let final_offset = (offset_y - 200.0).max(0.0);

                        return iced::widget::operation::scroll_to(
                            iced::widget::Id::new("calendar_scroll"),
                            iced::widget::scrollable::AbsoluteOffset {
                                x: None,
                                y: Some(final_offset),
                            },
                        )
                        .map(|_: ()| Message::NoOp);
                    }
                    Err(e) => {
                        self.calendar_error = Some(e);
                        let delay_secs =
                            Self::calendar_retry_delay_secs(self.calendar_retry_attempts);
                        self.calendar_retry_attempts =
                            self.calendar_retry_attempts.saturating_add(1).min(6);
                        self.calendar_next_retry = Some(now + Duration::from_secs(delay_secs));
                    }
                }
            }
            Message::Tick if self.is_calendar_open() && !self.calendar_loading => {
                let should_fetch = self
                    .calendar_last_fetch
                    .is_none_or(|last_fetch| last_fetch.elapsed().as_secs() >= 15 * 60);
                if should_fetch {
                    return self.request_calendar_refresh(false);
                }
            }
            _ => {}
        }

        Task::none()
    }
}
