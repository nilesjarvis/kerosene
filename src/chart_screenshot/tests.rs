use super::*;
use crate::chart::{OrderOverlay, PositionOverlay};
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;
use chrono::{Local, TimeZone};
use iced::Rectangle;

mod bitmap;
mod export;
mod io;
mod label;
mod sizing;

fn test_label_style() -> ChartScreenshotLabelStyle {
    ChartScreenshotLabelStyle {
        background: [8, 12, 18, 218],
        border: [255, 255, 255, 62],
        accent: [80, 250, 123, 210],
        text: [245, 248, 250, 245],
    }
}

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

fn dimensions_or_panic(dimensions: Option<(u32, u32)>, context: &str) -> (u32, u32) {
    match dimensions {
        Some(dimensions) => dimensions,
        None => panic!("{context}"),
    }
}

fn export_size_or_panic(result: Result<(u32, u32), String>, context: &str) -> (u32, u32) {
    match result {
        Ok(size) => size,
        Err(error) => panic!("{context}: {error}"),
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
