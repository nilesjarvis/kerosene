use super::model::{
    MarketSessionReturnBar, MarketSessionSummary, SessionReturnBar, SessionWeekday,
    SessionWeekdaySummary,
};
use crate::api::Candle;
use crate::market_sessions::{MarketSession, visible_session_ranges};
use chrono::{DateTime, Datelike, Utc};

#[cfg(test)]
pub(super) fn session_return_bars(candles: &[Candle]) -> Vec<SessionReturnBar> {
    let mut bars = candles
        .iter()
        .filter_map(session_return_bar)
        .collect::<Vec<_>>();
    bars.sort_by_key(|bar| bar.open_time);
    bars
}

pub(super) fn completed_session_return_bars(
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

pub(super) fn weekday_summaries(bars: &[SessionReturnBar]) -> Vec<SessionWeekdaySummary> {
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
            let (average_return_pct, win_rate_pct) =
                summary_rates(totals[idx], wins[idx], sample_count);

            SessionWeekdaySummary {
                weekday,
                sample_count,
                average_return_pct,
                win_rate_pct,
            }
        })
        .collect()
}

pub(super) fn empty_weekday_summaries() -> Vec<SessionWeekdaySummary> {
    weekday_summaries(&[])
}

/// Open-to-close returns for every market session band fully covered by the
/// intraday candles and completed by `completed_through_ms`. Bands follow the
/// chart's session ranges, tiling every day including weekends; partially
/// covered bands at either edge of the data are dropped.
pub(super) fn market_session_return_bars(
    intraday_candles: &[Candle],
    completed_through_ms: u64,
) -> Vec<MarketSessionReturnBar> {
    let mut candles = intraday_candles.iter().collect::<Vec<_>>();
    candles.sort_by_key(|candle| candle.open_time);
    candles.dedup_by_key(|candle| candle.open_time);

    let (Some(first), Some(last)) = (candles.first(), candles.last()) else {
        return Vec::new();
    };
    let data_start = first.open_time;
    let data_end = last.close_time.saturating_add(1).min(completed_through_ms);
    candles
        .retain(|candle| candle.open > 0.0 && candle.open.is_finite() && candle.close.is_finite());
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

pub(super) fn market_session_summaries(
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
            let (average_return_pct, win_rate_pct) = summary_rates(total, wins, sample_count);

            MarketSessionSummary {
                session,
                sample_count,
                average_return_pct,
                win_rate_pct,
            }
        })
        .collect()
}

pub(super) fn empty_market_session_summaries() -> Vec<MarketSessionSummary> {
    market_session_summaries(&[])
}

fn summary_rates(total_return_pct: f64, wins: usize, sample_count: usize) -> (f64, f64) {
    if sample_count > 0 {
        (
            total_return_pct / sample_count as f64,
            wins as f64 / sample_count as f64 * 100.0,
        )
    } else {
        (0.0, 0.0)
    }
}
