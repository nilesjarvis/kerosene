use super::*;
use crate::chart::{OrderOverlay, PositionOverlay};
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;
use chrono::{Local, TimeZone};

mod bitmap;
mod export;
mod io;

fn png_or_panic(result: Result<Vec<u8>, String>, context: &str) -> Vec<u8> {
    match result {
        Ok(png) => png,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn error_or_panic<T>(result: Result<T, String>, context: &str) -> String {
    match result {
        Ok(_) => panic!("{context}"),
        Err(error) => error,
    }
}

fn local_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> chrono::DateTime<Local> {
    match Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
    {
        Some(time) => time,
        None => panic!("valid local timestamp"),
    }
}
