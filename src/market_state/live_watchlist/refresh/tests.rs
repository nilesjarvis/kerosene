use super::*;

mod contexts;
mod force;
mod history;

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

fn plan(input: LiveWatchlistRefreshInput<'_>) -> LiveWatchlistRefreshPlan {
    plan_live_watchlist_refresh(input)
}
