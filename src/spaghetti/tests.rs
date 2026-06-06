use super::state::DEFAULT_PX_PER_MS;
use super::{SESSION_OPTIONS, Series, Session, SpaghettiCanvas};
use crate::api::Candle;
use chrono::TimeZone;
use iced::Color;

mod series;
mod sessions;
mod time_window;

fn ts_ms(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> u64 {
    let timestamp = chrono::Utc
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single();
    let Some(timestamp) = timestamp else {
        panic!("valid UTC timestamp");
    };

    match u64::try_from(timestamp.timestamp_millis()) {
        Ok(timestamp) => timestamp,
        Err(_) => panic!("non-negative timestamp"),
    }
}

fn candle_at(open_time: u64, close: f64) -> Candle {
    Candle::test_price(open_time, close)
}
