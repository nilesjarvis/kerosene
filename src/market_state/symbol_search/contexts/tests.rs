use crate::api::WatchlistContext;
use crate::market_state::SymbolSearchSortMode;

use super::*;
use std::collections::HashMap;

mod requests;
mod skips;

fn context() -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        day_vlm: Some(1.0),
    }
}

fn symbols() -> Vec<String> {
    vec!["BTC".to_string(), "ETH".to_string()]
}
