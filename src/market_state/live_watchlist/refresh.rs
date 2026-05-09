use crate::api::WatchlistContext;

use std::collections::HashMap;

mod contexts;
mod history;
#[cfg(test)]
mod tests;

use self::contexts::should_refresh_contexts;
use self::history::select_history_symbols;

// ---------------------------------------------------------------------------
// Live Watchlist Refresh Planner
// ---------------------------------------------------------------------------

const SLOW_DATA_REFRESH_MS: u64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LiveWatchlistRefreshPlan {
    pub(super) requested_at: u64,
    pub(super) context_symbols: Vec<String>,
    pub(super) history_symbols: Vec<String>,
}

pub(super) struct LiveWatchlistRefreshInput<'a> {
    pub(super) symbols: Vec<String>,
    pub(super) force: bool,
    pub(super) now_ms: u64,
    pub(super) contexts_last_fetch_ms: Option<u64>,
    pub(super) contexts: &'a HashMap<String, WatchlistContext>,
    pub(super) contexts_loading: bool,
    pub(super) history_loaded_at: &'a HashMap<String, u64>,
    pub(super) history_loading: bool,
}

impl LiveWatchlistRefreshPlan {
    pub(super) fn has_requests(&self) -> bool {
        !self.context_symbols.is_empty() || !self.history_symbols.is_empty()
    }
}

pub(super) fn plan_live_watchlist_refresh(
    input: LiveWatchlistRefreshInput<'_>,
) -> LiveWatchlistRefreshPlan {
    let context_symbols = if should_refresh_contexts(
        &input.symbols,
        input.force,
        input.now_ms,
        input.contexts_last_fetch_ms,
        input.contexts,
        input.contexts_loading,
    ) {
        input.symbols.clone()
    } else {
        Vec::new()
    };

    let history_symbols = select_history_symbols(
        input.symbols,
        input.force,
        input.now_ms,
        input.history_loaded_at,
        input.history_loading,
    );

    LiveWatchlistRefreshPlan {
        requested_at: input.now_ms,
        context_symbols,
        history_symbols,
    }
}
