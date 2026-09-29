use super::{model::*, returns::*, statistics::*};
use crate::api::Candle;
use crate::market_sessions::MarketSession;
use chrono::Utc;

mod returns;
mod statistics;

const HALF_HOUR_MS: u64 = 1_800_000;

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
