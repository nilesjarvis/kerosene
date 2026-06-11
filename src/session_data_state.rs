use crate::api::Candle;
use crate::market_sessions::{MarketSession, visible_session_ranges};
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MarketSessionReturnBar {
    pub(crate) kind: MarketSession,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
    pub(crate) open: f64,
    pub(crate) close: f64,
    pub(crate) return_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MarketSessionSummary {
    pub(crate) session: MarketSession,
    pub(crate) sample_count: usize,
    pub(crate) average_return_pct: f64,
    pub(crate) win_rate_pct: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionDataCandles {
    pub(crate) daily: Vec<Candle>,
    pub(crate) intraday: Vec<Candle>,
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
    pub(crate) session_summaries: Vec<MarketSessionSummary>,
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
            session_summaries: empty_market_session_summaries(),
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
        self.session_summaries = empty_market_session_summaries();
        self.loading = false;
        self.error = None;
        self.last_fetch_ms = None;
        self.pending_request = None;
    }

    pub(crate) fn apply_candles(&mut self, candles: SessionDataCandles, completed_through_ms: u64) {
        self.bars = completed_session_return_bars(&candles.daily, completed_through_ms);
        self.weekday_summaries = weekday_summaries(&self.bars);
        let session_bars = market_session_return_bars(&candles.intraday, completed_through_ms);
        self.session_summaries = market_session_summaries(&session_bars);
        self.candles = candles.daily;
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

/// Open-to-close returns for every market session band fully covered by the
/// intraday candles and completed by `completed_through_ms`. Bands follow the
/// chart's session ranges, tiling every day including weekends; partially
/// covered bands at either edge of the data are dropped.
pub(crate) fn market_session_return_bars(
    intraday_candles: &[Candle],
    completed_through_ms: u64,
) -> Vec<MarketSessionReturnBar> {
    let mut all_candles = intraday_candles.iter().collect::<Vec<_>>();
    all_candles.sort_by_key(|candle| candle.open_time);
    all_candles.dedup_by_key(|candle| candle.open_time);

    let (Some(first), Some(last)) = (all_candles.first(), all_candles.last()) else {
        return Vec::new();
    };
    let data_start = first.open_time;
    let data_end = last.close_time.saturating_add(1).min(completed_through_ms);
    let candles = all_candles
        .into_iter()
        .filter(|candle| candle.open > 0.0 && candle.open.is_finite() && candle.close.is_finite())
        .collect::<Vec<_>>();
    if candles.is_empty() {
        return Vec::new();
    }

    visible_session_ranges(data_start, data_end)
        .into_iter()
        .filter(|range| range.start_ms >= data_start && range.end_ms <= data_end)
        .filter_map(|range| {
            let begin = candles.partition_point(|candle| candle.open_time < range.start_ms);
            let end = candles.partition_point(|candle| candle.open_time < range.end_ms);
            if begin >= end {
                return None;
            }
            let open = candles[begin].open;
            let close = candles[end - 1].close;
            let return_pct = ((close - open) / open) * 100.0;
            return_pct.is_finite().then_some(MarketSessionReturnBar {
                kind: range.kind,
                start_ms: range.start_ms,
                end_ms: range.end_ms,
                open,
                close,
                return_pct,
            })
        })
        .collect()
}

pub(crate) fn market_session_summaries(
    bars: &[MarketSessionReturnBar],
) -> Vec<MarketSessionSummary> {
    MarketSession::ALL
        .into_iter()
        .map(|session| {
            let mut sample_count = 0_usize;
            let mut total = 0.0_f64;
            let mut wins = 0_usize;
            for bar in bars.iter().filter(|bar| bar.kind == session) {
                sample_count += 1;
                total += bar.return_pct;
                if bar.return_pct > 0.0 {
                    wins += 1;
                }
            }
            let average_return_pct = if sample_count > 0 {
                total / sample_count as f64
            } else {
                0.0
            };
            let win_rate_pct = if sample_count > 0 {
                wins as f64 / sample_count as f64 * 100.0
            } else {
                0.0
            };

            MarketSessionSummary {
                session,
                sample_count,
                average_return_pct,
                win_rate_pct,
            }
        })
        .collect()
}

fn empty_market_session_summaries() -> Vec<MarketSessionSummary> {
    market_session_summaries(&[])
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

    const HALF_HOUR_MS: u64 = 1_800_000;

    fn ts(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> u64 {
        use chrono::TimeZone;
        u64::try_from(
            Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
                .single()
                .expect("valid UTC timestamp")
                .timestamp_millis(),
        )
        .expect("positive timestamp")
    }

    fn half_hour_candles(start_ms: u64, end_ms: u64, price_at: impl Fn(u64) -> f64) -> Vec<Candle> {
        let mut candles = Vec::new();
        let mut open_time = start_ms;
        while open_time < end_ms {
            let open = price_at(open_time);
            let close = price_at(open_time + HALF_HOUR_MS);
            candles.push(Candle::test_ohlcv(
                open_time,
                open_time + HALF_HOUR_MS - 1,
                [open, open.max(close), open.min(close), close],
                50.0,
            ));
            open_time += HALF_HOUR_MS;
        }
        candles
    }

    fn hours_price(day_start_ms: u64) -> impl Fn(u64) -> f64 {
        move |time_ms| 100.0 + (time_ms.saturating_sub(day_start_ms)) as f64 / 3_600_000.0
    }

    #[test]
    fn market_session_returns_split_intraday_candles_by_session_band() {
        // 2026-01-14 (winter offsets): Asia 00:00-08:00, London 08:00-14:30,
        // New York 14:30-21:00, Overnight 21:00-00:00 UTC.
        let day_start = ts(2026, 1, 14, 0, 0);
        let day_end = ts(2026, 1, 15, 0, 0);
        let candles = half_hour_candles(day_start, day_end, hours_price(day_start));

        let bars = market_session_return_bars(&candles, day_end);

        let kinds = bars.iter().map(|bar| bar.kind).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                MarketSession::Asia,
                MarketSession::London,
                MarketSession::NewYork,
                MarketSession::Overnight,
            ]
        );
        assert_eq!(bars[0].start_ms, day_start);
        assert_eq!(bars[0].end_ms, ts(2026, 1, 14, 8, 0));
        crate::helpers::assert_close(bars[0].return_pct, 8.0);
        crate::helpers::assert_close(bars[1].return_pct, (114.5 - 108.0) / 108.0 * 100.0);
        crate::helpers::assert_close(bars[2].return_pct, (121.0 - 114.5) / 114.5 * 100.0);
        crate::helpers::assert_close(bars[3].return_pct, (124.0 - 121.0) / 121.0 * 100.0);
    }

    #[test]
    fn market_session_returns_drop_partially_covered_bands() {
        let day_start = ts(2026, 1, 14, 0, 0);
        // Data starts after the Asia open and the clock stops before the
        // Overnight band completes, so only London and New York qualify.
        let candles = half_hour_candles(
            ts(2026, 1, 14, 1, 0),
            ts(2026, 1, 15, 0, 0),
            hours_price(day_start),
        );

        let bars = market_session_return_bars(&candles, ts(2026, 1, 14, 22, 0));

        let kinds = bars.iter().map(|bar| bar.kind).collect::<Vec<_>>();
        assert_eq!(kinds, vec![MarketSession::London, MarketSession::NewYork]);
    }

    #[test]
    fn market_session_returns_skip_invalid_candles() {
        let day_start = ts(2026, 1, 14, 0, 0);
        let price_at = hours_price(day_start);
        let mut candles = half_hour_candles(day_start, ts(2026, 1, 15, 0, 0), &price_at);
        // Corrupt the candle at the 08:00 London open; the band open falls
        // back to the 08:30 candle while the Asia band stays intact.
        candles[16].open = f64::NAN;

        let bars = market_session_return_bars(&candles, ts(2026, 1, 15, 0, 0));

        assert_eq!(bars[0].kind, MarketSession::Asia);
        crate::helpers::assert_close(bars[0].return_pct, 8.0);
        assert_eq!(bars[1].kind, MarketSession::London);
        crate::helpers::assert_close(bars[1].open, price_at(ts(2026, 1, 14, 8, 30)));
        crate::helpers::assert_close(bars[1].return_pct, (114.5 - 108.5) / 108.5 * 100.0);
    }

    #[test]
    fn market_session_summaries_track_average_count_and_win_rate() {
        let band = |kind, start_ms: u64, return_pct: f64| MarketSessionReturnBar {
            kind,
            start_ms,
            end_ms: start_ms + HALF_HOUR_MS,
            open: 100.0,
            close: 100.0 + return_pct,
            return_pct,
        };
        let bars = vec![
            band(MarketSession::Asia, 0, 2.0),
            band(MarketSession::Asia, 1, -1.0),
            band(MarketSession::London, 2, 1.0),
        ];

        let summaries = market_session_summaries(&bars);

        assert_eq!(summaries.len(), MarketSession::ALL.len());
        for (idx, session) in MarketSession::ALL.into_iter().enumerate() {
            assert_eq!(summaries[idx].session, session);
        }

        let asia = &summaries[0];
        assert_eq!(asia.sample_count, 2);
        crate::helpers::assert_close(asia.average_return_pct, 0.5);
        crate::helpers::assert_close(asia.win_rate_pct, 50.0);

        let london = &summaries[1];
        assert_eq!(london.sample_count, 1);
        crate::helpers::assert_close(london.average_return_pct, 1.0);
        crate::helpers::assert_close(london.win_rate_pct, 100.0);

        let new_york = &summaries[2];
        assert_eq!(new_york.sample_count, 0);
        crate::helpers::assert_close(new_york.average_return_pct, 0.0);
        crate::helpers::assert_close(new_york.win_rate_pct, 0.0);
    }
}
