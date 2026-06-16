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

impl SessionDataRequest {
    pub(crate) fn matches_refresh_target(
        &self,
        id: SessionDataId,
        symbol: &str,
        lookback: SessionDataLookback,
    ) -> bool {
        self.id == id && self.symbol == symbol && self.lookback == lookback
    }
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

// ---------------------------------------------------------------------------
// Summary statistics (verdict line + KPI strip)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionGroup {
    Weekday,
    Session,
}

/// One eligible bucket that can headline the verdict line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VerdictBucket {
    pub(crate) group: SessionGroup,
    pub(crate) label: String,
    pub(crate) average_return_pct: f64,
    pub(crate) win_rate_pct: f64,
    pub(crate) sample_count: usize,
}

/// The plain-language headline for the widget: either the strongest/weakest
/// eligible buckets, or an explicit "not enough data" state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SessionVerdict {
    Insufficient {
        total_samples: usize,
        min_required: usize,
    },
    Edge {
        strongest: VerdictBucket,
        weakest: Option<VerdictBucket>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionStreak {
    pub(crate) length: usize,
    pub(crate) positive: bool,
}

/// Minimum completed-session count for a bucket to be eligible for the verdict.
/// Scales gently with the sample size so a single fluke is never crowned.
pub(crate) fn verdict_min_samples(total_samples: usize) -> usize {
    (total_samples / 40).max(4)
}

/// Count-weighted overall win rate across the weekday buckets — equivalent to
/// total green sessions / total sessions. `None` when there are no samples.
pub(crate) fn overall_win_rate_pct(weekday_summaries: &[SessionWeekdaySummary]) -> Option<f64> {
    let mut total = 0usize;
    let mut weighted = 0.0f64;
    for summary in weekday_summaries {
        total += summary.sample_count;
        weighted += summary.win_rate_pct * summary.sample_count as f64;
    }
    (total > 0).then(|| weighted / total as f64)
}

/// Mean absolute open-to-close move across completed sessions. `None` when empty.
pub(crate) fn average_abs_move_pct(bars: &[SessionReturnBar]) -> Option<f64> {
    if bars.is_empty() {
        return None;
    }
    let sum: f64 = bars.iter().map(|bar| bar.return_pct.abs()).sum();
    let avg = sum / bars.len() as f64;
    avg.is_finite().then_some(avg)
}

/// Compounded total return across all completed sessions, in percent.
pub(crate) fn total_return_pct(bars: &[SessionReturnBar]) -> Option<f64> {
    if bars.is_empty() {
        return None;
    }
    let growth = bars
        .iter()
        .fold(1.0f64, |acc, bar| acc * (1.0 + bar.return_pct / 100.0));
    let total = (growth - 1.0) * 100.0;
    total.is_finite().then_some(total)
}

/// Trailing run of same-signed sessions, counted from the most recent bar.
/// `bars` is assumed sorted ascending by open time. A flat (0%) latest session
/// yields `None`.
pub(crate) fn current_streak(bars: &[SessionReturnBar]) -> Option<SessionStreak> {
    let last = bars.last()?;
    let positive = if last.return_pct > 0.0 {
        true
    } else if last.return_pct < 0.0 {
        false
    } else {
        return None;
    };
    let length = bars
        .iter()
        .rev()
        .take_while(|bar| {
            if positive {
                bar.return_pct > 0.0
            } else {
                bar.return_pct < 0.0
            }
        })
        .count();
    (length > 0).then_some(SessionStreak { length, positive })
}

/// Weekday with the greatest total traded volume over the lookback. `None` when
/// there is no positive volume to compare.
pub(crate) fn most_active_weekday(bars: &[SessionReturnBar]) -> Option<SessionWeekday> {
    let mut totals = [0.0f64; 7];
    for bar in bars {
        if bar.volume.is_finite() && bar.volume > 0.0 {
            totals[bar.weekday.index()] += bar.volume;
        }
    }
    let mut best: Option<(usize, f64)> = None;
    for (idx, &total) in totals.iter().enumerate() {
        if total > 0.0 && best.is_none_or(|(_, current)| total > current) {
            best = Some((idx, total));
        }
    }
    best.map(|(idx, _)| SessionWeekday::ALL[idx])
}

/// Sample standard deviation of open-to-close returns per weekday, indexed by
/// `SessionWeekday::index`. `None` for weekdays with fewer than two samples.
pub(crate) fn weekday_dispersions(bars: &[SessionReturnBar]) -> [Option<f64>; 7] {
    let mut sums = [0.0f64; 7];
    let mut sum_sqs = [0.0f64; 7];
    let mut counts = [0usize; 7];
    for bar in bars {
        let idx = bar.weekday.index();
        sums[idx] += bar.return_pct;
        sum_sqs[idx] += bar.return_pct * bar.return_pct;
        counts[idx] += 1;
    }
    let mut out = [None; 7];
    for (idx, &n) in counts.iter().enumerate() {
        if n >= 2 {
            let n_f = n as f64;
            let sum = sums[idx];
            let variance = (sum_sqs[idx] - sum * sum / n_f) / (n_f - 1.0);
            let std = variance.max(0.0).sqrt();
            if std.is_finite() {
                out[idx] = Some(std);
            }
        }
    }
    out
}

/// The strongest, and when distinct the weakest, eligible bucket across both the
/// weekday and market-session breakdowns. Eligibility is gated by
/// [`verdict_min_samples`] so thin buckets never headline.
pub(crate) fn session_verdict(
    weekday_summaries: &[SessionWeekdaySummary],
    session_summaries: &[MarketSessionSummary],
    total_samples: usize,
) -> SessionVerdict {
    let min_required = verdict_min_samples(total_samples);
    let mut buckets: Vec<VerdictBucket> = Vec::new();
    for summary in weekday_summaries {
        if summary.sample_count >= min_required {
            buckets.push(VerdictBucket {
                group: SessionGroup::Weekday,
                label: summary.weekday.label().to_string(),
                average_return_pct: summary.average_return_pct,
                win_rate_pct: summary.win_rate_pct,
                sample_count: summary.sample_count,
            });
        }
    }
    for summary in session_summaries {
        if summary.sample_count >= min_required {
            buckets.push(VerdictBucket {
                group: SessionGroup::Session,
                label: summary.session.short_label().to_string(),
                average_return_pct: summary.average_return_pct,
                win_rate_pct: summary.win_rate_pct,
                sample_count: summary.sample_count,
            });
        }
    }

    if buckets.is_empty() {
        return SessionVerdict::Insufficient {
            total_samples,
            min_required,
        };
    }

    let strongest_idx = buckets
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.average_return_pct.total_cmp(&b.average_return_pct))
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let weakest_idx = buckets
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.average_return_pct.total_cmp(&b.average_return_pct))
        .map(|(idx, _)| idx)
        .unwrap_or(0);

    let strongest = buckets[strongest_idx].clone();
    let weakest = (weakest_idx != strongest_idx).then(|| buckets[weakest_idx].clone());
    SessionVerdict::Edge { strongest, weakest }
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

    fn sample_bar(
        weekday: SessionWeekday,
        return_pct: f64,
        volume: f64,
        open_time: u64,
    ) -> SessionReturnBar {
        SessionReturnBar {
            open_time,
            close_time: open_time + 1,
            weekday,
            open: 100.0,
            close: 100.0 * (1.0 + return_pct / 100.0),
            volume,
            return_pct,
        }
    }

    fn weekday_summary(
        weekday: SessionWeekday,
        sample_count: usize,
        average_return_pct: f64,
        win_rate_pct: f64,
    ) -> SessionWeekdaySummary {
        SessionWeekdaySummary {
            weekday,
            sample_count,
            average_return_pct,
            win_rate_pct,
        }
    }

    #[test]
    fn overall_win_rate_is_count_weighted() {
        let summaries = vec![
            weekday_summary(SessionWeekday::Mon, 2, 0.0, 50.0),
            weekday_summary(SessionWeekday::Tue, 8, 0.0, 75.0),
        ];
        // (50*2 + 75*8) / 10 = 70
        crate::helpers::assert_close(overall_win_rate_pct(&summaries).unwrap(), 70.0);
        assert!(overall_win_rate_pct(&[]).is_none());
        assert!(
            overall_win_rate_pct(&[weekday_summary(SessionWeekday::Mon, 0, 0.0, 0.0)]).is_none()
        );
    }

    #[test]
    fn average_abs_move_uses_magnitude() {
        let bars = vec![
            sample_bar(SessionWeekday::Mon, 2.0, 1.0, 0),
            sample_bar(SessionWeekday::Tue, -4.0, 1.0, 1),
        ];
        crate::helpers::assert_close(average_abs_move_pct(&bars).unwrap(), 3.0);
        assert!(average_abs_move_pct(&[]).is_none());
    }

    #[test]
    fn total_return_compounds_sessions() {
        let bars = vec![
            sample_bar(SessionWeekday::Mon, 10.0, 1.0, 0),
            sample_bar(SessionWeekday::Tue, -10.0, 1.0, 1),
        ];
        // 1.10 * 0.90 - 1 = -0.01 -> -1%
        crate::helpers::assert_close(total_return_pct(&bars).unwrap(), -1.0);
        assert!(total_return_pct(&[]).is_none());
    }

    #[test]
    fn current_streak_counts_trailing_same_sign() {
        let bars = vec![
            sample_bar(SessionWeekday::Mon, 1.0, 1.0, 0),
            sample_bar(SessionWeekday::Tue, -2.0, 1.0, 1),
            sample_bar(SessionWeekday::Wed, 3.0, 1.0, 2),
            sample_bar(SessionWeekday::Thu, 4.0, 1.0, 3),
        ];
        let streak = current_streak(&bars).expect("trailing up streak");
        assert_eq!(streak.length, 2);
        assert!(streak.positive);
    }

    #[test]
    fn current_streak_none_when_latest_flat() {
        let bars = vec![
            sample_bar(SessionWeekday::Mon, 3.0, 1.0, 0),
            sample_bar(SessionWeekday::Tue, 0.0, 1.0, 1),
        ];
        assert!(current_streak(&bars).is_none());
        assert!(current_streak(&[]).is_none());
    }

    #[test]
    fn most_active_weekday_picks_max_volume() {
        let bars = vec![
            sample_bar(SessionWeekday::Mon, 1.0, 10.0, 0),
            sample_bar(SessionWeekday::Tue, 1.0, 50.0, 1),
            sample_bar(SessionWeekday::Mon, 1.0, 30.0, 2),
        ];
        // Mon totals 40, Tue 50 -> Tue
        assert_eq!(most_active_weekday(&bars), Some(SessionWeekday::Tue));
        assert!(most_active_weekday(&[]).is_none());
    }

    #[test]
    fn weekday_dispersions_use_sample_std_dev() {
        let bars = vec![
            sample_bar(SessionWeekday::Mon, 1.0, 1.0, 0),
            sample_bar(SessionWeekday::Mon, 3.0, 1.0, 1),
            sample_bar(SessionWeekday::Tue, 5.0, 1.0, 2),
        ];
        let disp = weekday_dispersions(&bars);
        // Mon mean 2, sample variance (1 + 1)/(2-1) = 2 -> std sqrt(2)
        crate::helpers::assert_close(disp[SessionWeekday::Mon.index()].unwrap(), 2.0_f64.sqrt());
        // Tue single sample -> None
        assert!(disp[SessionWeekday::Tue.index()].is_none());
    }

    #[test]
    fn session_verdict_picks_strongest_and_weakest_eligible() {
        let weekdays = vec![
            // Highest avg but below MIN_N -> must be ignored.
            weekday_summary(SessionWeekday::Mon, 2, 5.0, 100.0),
            weekday_summary(SessionWeekday::Tue, 10, 0.8, 62.0),
            weekday_summary(SessionWeekday::Sun, 10, -0.6, 40.0),
        ];
        match session_verdict(&weekdays, &[], 24) {
            SessionVerdict::Edge { strongest, weakest } => {
                assert_eq!(strongest.label, "Tue");
                crate::helpers::assert_close(strongest.average_return_pct, 0.8);
                let weakest = weakest.expect("distinct weakest");
                assert_eq!(weakest.label, "Sun");
                crate::helpers::assert_close(weakest.average_return_pct, -0.6);
            }
            other => panic!("expected Edge, got {other:?}"),
        }
    }

    #[test]
    fn session_verdict_insufficient_when_no_bucket_clears_min() {
        let weekdays = vec![
            weekday_summary(SessionWeekday::Mon, 3, 5.0, 100.0),
            weekday_summary(SessionWeekday::Tue, 2, -5.0, 0.0),
        ];
        // total_samples 5 -> min_required max(4, 0) = 4; nothing qualifies.
        assert!(matches!(
            session_verdict(&weekdays, &[], 5),
            SessionVerdict::Insufficient {
                min_required: 4,
                ..
            }
        ));
    }

    #[test]
    fn session_verdict_single_eligible_has_no_weakest() {
        let weekdays = vec![weekday_summary(SessionWeekday::Tue, 10, 0.8, 62.0)];
        match session_verdict(&weekdays, &[], 10) {
            SessionVerdict::Edge { strongest, weakest } => {
                assert_eq!(strongest.label, "Tue");
                assert!(weakest.is_none());
            }
            other => panic!("expected Edge, got {other:?}"),
        }
    }
}
