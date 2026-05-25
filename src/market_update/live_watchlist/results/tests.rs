use crate::api::WatchlistContext;

use super::*;
use std::collections::HashMap;

mod contexts;
mod history;

fn context(day_vlm: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        day_vlm: Some(day_vlm),
    }
}
