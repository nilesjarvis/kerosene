use crate::api::WatchlistContext;

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Live Watchlist Results
// ---------------------------------------------------------------------------

pub(super) fn apply_contexts_loaded(
    loading: &mut bool,
    last_fetch_ms: &mut Option<u64>,
    contexts: &mut HashMap<String, WatchlistContext>,
    status: &mut Option<(String, bool)>,
    requested_at: u64,
    result: Result<HashMap<String, WatchlistContext>, String>,
) {
    *loading = false;
    *last_fetch_ms = Some(requested_at);

    match result {
        Ok(loaded_contexts) => {
            *contexts = loaded_contexts;
        }
        Err(error) => {
            *status = Some((format!("Watchlist context refresh failed: {error}"), true));
        }
    }
}

pub(super) fn apply_history_loaded(
    loading: &mut bool,
    loaded_at: &mut HashMap<String, u64>,
    history: &mut HashMap<String, (f64, f64, f64)>,
    status: &mut Option<(String, bool)>,
    requested_symbols: Vec<String>,
    requested_at: u64,
    result: Result<HashMap<String, (f64, f64, f64)>, String>,
) {
    *loading = false;
    for symbol in requested_symbols {
        loaded_at.insert(symbol, requested_at);
    }

    match result {
        Ok(loaded_history) => {
            history.extend(loaded_history);
        }
        Err(error) => {
            *status = Some((format!("Watchlist history refresh failed: {error}"), true));
        }
    }
}
