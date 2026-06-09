use crate::api::Candle;
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

pub(crate) type SessionDataId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum SessionDataLookback {
    #[default]
    FourWeeks,
    EightWeeks,
    ThreeMonths,
    SixMonths,
    OneYear,
}

impl SessionDataLookback {
    pub(crate) const ALL: [Self; 5] = [
        Self::FourWeeks,
        Self::EightWeeks,
        Self::ThreeMonths,
        Self::SixMonths,
        Self::OneYear,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::FourWeeks => "4W",
            Self::EightWeeks => "8W",
            Self::ThreeMonths => "3M",
            Self::SixMonths => "6M",
            Self::OneYear => "1Y",
        }
    }

    pub(crate) fn days(self) -> u64 {
        match self {
            Self::FourWeeks => 28,
            Self::EightWeeks => 56,
            Self::ThreeMonths => 90,
            Self::SixMonths => 180,
            Self::OneYear => 365,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionWeekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl SessionWeekday {
    pub(crate) const ALL: [Self; 7] = [
        Self::Mon,
        Self::Tue,
        Self::Wed,
        Self::Thu,
        Self::Fri,
        Self::Sat,
        Self::Sun,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Mon => "Mon",
            Self::Tue => "Tue",
            Self::Wed => "Wed",
            Self::Thu => "Thu",
            Self::Fri => "Fri",
            Self::Sat => "Sat",
            Self::Sun => "Sun",
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            Self::Mon => 0,
            Self::Tue => 1,
            Self::Wed => 2,
            Self::Thu => 3,
            Self::Fri => 4,
            Self::Sat => 5,
            Self::Sun => 6,
        }
    }

    fn from_chrono(value: chrono::Weekday) -> Self {
        match value {
            chrono::Weekday::Mon => Self::Mon,
            chrono::Weekday::Tue => Self::Tue,
            chrono::Weekday::Wed => Self::Wed,
            chrono::Weekday::Thu => Self::Thu,
            chrono::Weekday::Fri => Self::Fri,
            chrono::Weekday::Sat => Self::Sat,
            chrono::Weekday::Sun => Self::Sun,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionReturnBar {
    pub(crate) open_time: u64,
    pub(crate) close_time: u64,
    pub(crate) weekday: SessionWeekday,
    pub(crate) open: f64,
    pub(crate) close: f64,
    pub(crate) volume: f64,
    pub(crate) return_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionWeekdaySummary {
    pub(crate) weekday: SessionWeekday,
    pub(crate) sample_count: usize,
    pub(crate) average_return_pct: f64,
    pub(crate) win_rate_pct: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionDataRequest {
    pub(crate) id: SessionDataId,
    pub(crate) symbol: String,
    pub(crate) lookback: SessionDataLookback,
    pub(crate) requested_at_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionDataInstance {
    pub(crate) id: SessionDataId,
    pub(crate) symbol: String,
    pub(crate) search_query: String,
    pub(crate) symbol_picker_open: bool,
    pub(crate) lookback: SessionDataLookback,
    pub(crate) candles: Vec<Candle>,
    pub(crate) bars: Vec<SessionReturnBar>,
    pub(crate) weekday_summaries: Vec<SessionWeekdaySummary>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) last_fetch_ms: Option<u64>,
    pub(crate) pending_request: Option<SessionDataRequest>,
}

impl SessionDataInstance {
    pub(crate) fn new(id: SessionDataId, symbol: String, lookback: SessionDataLookback) -> Self {
        Self {
            id,
            symbol,
            search_query: String::new(),
            symbol_picker_open: false,
            lookback,
            candles: Vec::new(),
            bars: Vec::new(),
            weekday_summaries: empty_weekday_summaries(),
            loading: false,
            error: None,
            last_fetch_ms: None,
            pending_request: None,
        }
    }

    pub(crate) fn clear_history(&mut self) {
        self.candles.clear();
        self.bars.clear();
        self.weekday_summaries = empty_weekday_summaries();
        self.loading = false;
        self.error = None;
        self.last_fetch_ms = None;
        self.pending_request = None;
    }

    pub(crate) fn apply_candles(&mut self, candles: Vec<Candle>, completed_through_ms: u64) {
        self.bars = completed_session_return_bars(&candles, completed_through_ms);
        self.weekday_summaries = weekday_summaries(&self.bars);
        self.candles = candles;
    }
}

#[cfg(test)]
fn session_return_bars(candles: &[Candle]) -> Vec<SessionReturnBar> {
    let mut bars = candles
        .iter()
        .filter_map(session_return_bar)
        .collect::<Vec<_>>();
    bars.sort_by_key(|bar| bar.open_time);
    bars
}

pub(crate) fn completed_session_return_bars(
    candles: &[Candle],
    completed_through_ms: u64,
) -> Vec<SessionReturnBar> {
    let mut bars = candles
        .iter()
        .filter(|candle| candle.close_time <= completed_through_ms)
        .filter_map(session_return_bar)
        .collect::<Vec<_>>();
    bars.sort_by_key(|bar| bar.open_time);
    bars
}

fn session_return_bar(candle: &Candle) -> Option<SessionReturnBar> {
    if candle.open <= 0.0
        || !candle.open.is_finite()
        || !candle.close.is_finite()
        || !candle.volume.is_finite()
    {
        return None;
    }
    let timestamp = i64::try_from(candle.open_time).ok()?;
    let date = DateTime::<Utc>::from_timestamp_millis(timestamp)?;
    let return_pct = ((candle.close - candle.open) / candle.open) * 100.0;
    return_pct.is_finite().then_some(SessionReturnBar {
        open_time: candle.open_time,
        close_time: candle.close_time,
        weekday: SessionWeekday::from_chrono(date.weekday()),
        open: candle.open,
        close: candle.close,
        volume: candle.volume,
        return_pct,
    })
}

pub(crate) fn weekday_summaries(bars: &[SessionReturnBar]) -> Vec<SessionWeekdaySummary> {
    let mut totals = [0.0_f64; 7];
    let mut counts = [0_usize; 7];
    let mut wins = [0_usize; 7];

    for bar in bars {
        let idx = bar.weekday.index();
        totals[idx] += bar.return_pct;
        counts[idx] += 1;
        if bar.return_pct > 0.0 {
            wins[idx] += 1;
        }
    }

    SessionWeekday::ALL
        .into_iter()
        .map(|weekday| {
            let idx = weekday.index();
            let sample_count = counts[idx];
            let average_return_pct = if sample_count > 0 {
                totals[idx] / sample_count as f64
            } else {
                0.0
            };
            let win_rate_pct = if sample_count > 0 {
                wins[idx] as f64 / sample_count as f64 * 100.0
            } else {
                0.0
            };

            SessionWeekdaySummary {
                weekday,
                sample_count,
                average_return_pct,
                win_rate_pct,
            }
        })
        .collect()
}

fn empty_weekday_summaries() -> Vec<SessionWeekdaySummary> {
    weekday_summaries(&[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Candle;

    fn candle(day_offset: u64, open: f64, close: f64) -> Candle {
        let open_time = 1_704_067_200_000 + day_offset * 86_400_000;
        Candle::test_ohlcv(
            open_time,
            open_time + 86_399_999,
            [open, open.max(close), open.min(close), close],
            100.0,
        )
    }

    fn candle_with_close_time(day_offset: u64, close_time: u64, open: f64, close: f64) -> Candle {
        let open_time = 1_704_067_200_000 + day_offset * 86_400_000;
        Candle::test_ohlcv(
            open_time,
            close_time,
            [open, open.max(close), open.min(close), close],
            100.0,
        )
    }

    #[test]
    fn session_returns_use_utc_open_weekday_and_open_to_close_return() {
        let bars = session_return_bars(&[candle(0, 100.0, 105.0)]);

        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].weekday, SessionWeekday::Mon);
        crate::helpers::assert_close(bars[0].return_pct, 5.0);
    }

    #[test]
    fn session_returns_skip_invalid_open_values() {
        let bars = session_return_bars(&[
            candle(0, 0.0, 105.0),
            candle(1, f64::NAN, 105.0),
            candle(2, 100.0, 99.0),
        ]);

        assert_eq!(bars.len(), 1);
        crate::helpers::assert_close(bars[0].return_pct, -1.0);
    }

    #[test]
    fn completed_session_returns_skip_open_daily_candle() {
        let completed = candle(0, 100.0, 105.0);
        let partial = candle_with_close_time(1, 1_704_239_999_999, 100.0, 90.0);
        let bars = completed_session_return_bars(&[completed, partial], 1_704_157_200_000);

        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].weekday, SessionWeekday::Mon);
        crate::helpers::assert_close(bars[0].return_pct, 5.0);
    }

    #[test]
    fn weekday_summaries_track_average_count_and_win_rate() {
        let bars = session_return_bars(&[
            candle(0, 100.0, 110.0),
            candle(7, 100.0, 90.0),
            candle(1, 100.0, 102.0),
        ]);

        let summaries = weekday_summaries(&bars);
        let monday = &summaries[SessionWeekday::Mon.index()];
        let tuesday = &summaries[SessionWeekday::Tue.index()];

        assert_eq!(monday.sample_count, 2);
        crate::helpers::assert_close(monday.average_return_pct, 0.0);
        crate::helpers::assert_close(monday.win_rate_pct, 50.0);
        assert_eq!(tuesday.sample_count, 1);
        crate::helpers::assert_close(tuesday.average_return_pct, 2.0);
        crate::helpers::assert_close(tuesday.win_rate_pct, 100.0);
    }

    #[test]
    fn weekday_summaries_include_all_weekdays() {
        let bars = session_return_bars(&[candle(0, 100.0, 110.0)]);

        let summaries = weekday_summaries(&bars);

        assert_eq!(summaries.len(), SessionWeekday::ALL.len());
        for (idx, weekday) in SessionWeekday::ALL.into_iter().enumerate() {
            assert_eq!(summaries[idx].weekday, weekday);
        }

        let monday = &summaries[SessionWeekday::Mon.index()];
        let tuesday = &summaries[SessionWeekday::Tue.index()];
        assert_eq!(monday.sample_count, 1);
        crate::helpers::assert_close(monday.average_return_pct, 10.0);
        assert_eq!(tuesday.sample_count, 0);
        crate::helpers::assert_close(tuesday.average_return_pct, 0.0);
        crate::helpers::assert_close(tuesday.win_rate_pct, 0.0);
    }
}
