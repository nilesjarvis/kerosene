use super::{MarketSession, calendar_data, format_market_time_until, local_datetime_for_date};
use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};
use chrono_tz::Tz;

type HourMinute = (u32, u32);
pub(super) type ExceptionTable = &'static [((i32, u32, u32), DateException)];

#[derive(Clone, Copy)]
pub(super) enum DateException {
    Holiday(&'static str),
    Hours(HourMinute, HourMinute),
}

enum TradingDay {
    Closed(Option<&'static str>),
    Session {
        open: DateTime<Tz>,
        close: DateTime<Tz>,
    },
}

pub(crate) enum MarketClockState {
    Open {
        close: DateTime<Tz>,
    },
    Closed {
        next_open: Option<DateTime<Tz>>,
        holiday: Option<&'static str>,
    },
    Unavailable,
}

pub(crate) struct MarketClock {
    pub(crate) session: MarketSession,
    pub(crate) state: MarketClockState,
    local_now: DateTime<Tz>,
}

impl MarketSession {
    fn trading_day(self, date: NaiveDate) -> Option<TradingDay> {
        if !calendar_data::YEARS.contains(&date.year()) {
            return None;
        }
        let exceptions = match self {
            Self::NewYork => calendar_data::XNYS,
            Self::London => calendar_data::XLON,
            Self::Asia => calendar_data::XTKS,
            Self::Overnight => return None,
        };
        if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            return Some(TradingDay::Closed(None));
        }
        let key = (date.year(), date.month(), date.day());
        let (open, close) = match exceptions.binary_search_by_key(&key, |(date, _)| *date) {
            Ok(index) => match exceptions[index].1 {
                DateException::Holiday(name) => return Some(TradingDay::Closed(Some(name))),
                DateException::Hours(open, close) => (open, close),
            },
            Err(_) => (self.open_hm(), self.close_hm()?),
        };
        let tz = self.timezone();
        Some(TradingDay::Session {
            open: local_datetime_for_date(tz, date, open.0, open.1)?,
            close: local_datetime_for_date(tz, date, close.0, close.1)?,
        })
    }

    fn next_open_after(self, mut date: NaiveDate) -> Option<DateTime<Tz>> {
        loop {
            date = date.succ_opt()?;
            if let TradingDay::Session { open, .. } = self.trading_day(date)? {
                return Some(open);
            }
        }
    }

    pub(crate) fn clock_at(self, now: DateTime<Utc>) -> MarketClock {
        let local_now = now.with_timezone(&self.timezone());
        let date = local_now.date_naive();
        let state = match self.trading_day(date) {
            None => MarketClockState::Unavailable,
            Some(TradingDay::Session { open, .. }) if local_now < open => {
                MarketClockState::Closed {
                    next_open: Some(open),
                    holiday: None,
                }
            }
            Some(TradingDay::Session { close, .. }) if local_now < close => {
                MarketClockState::Open { close }
            }
            day => MarketClockState::Closed {
                next_open: self.next_open_after(date),
                holiday: match day {
                    Some(TradingDay::Closed(holiday)) => holiday,
                    _ => None,
                },
            },
        };
        MarketClock {
            session: self,
            state,
            local_now,
        }
    }
}

impl MarketClock {
    pub(crate) fn tooltip_line(&self) -> String {
        let countdown = |bell: DateTime<Tz>| {
            format_market_time_until(self.local_now.with_timezone(&Utc), bell.with_timezone(&Utc))
        };
        let (status, detail) = match self.state {
            MarketClockState::Open { close } => ("OPEN", format!("closes in {}", countdown(close))),
            MarketClockState::Closed { next_open, holiday } => {
                let next = next_open.map_or_else(
                    || "next open unavailable".to_string(),
                    |open| format!("opens in {}", countdown(open)),
                );
                let detail = match holiday {
                    Some(name) => format!("{name}, {next}"),
                    None => next,
                };
                ("CLOSED", detail)
            }
            MarketClockState::Unavailable => ("UNKNOWN", "calendar unavailable".to_string()),
        };
        format!(
            "{status}  {} {} ({detail})",
            self.session.label(),
            self.local_now.format("%H:%M:%S %Z")
        )
    }
}
