use crate::api::{WatchlistContext, WatchlistContextsResponse};
use crate::helpers::redact_sensitive_response_text;

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Live Watchlist Results
// ---------------------------------------------------------------------------

const WATCHLIST_CONTEXT_FAILURE_PREFIX: &str = "Watchlist context refresh failed:";
const WATCHLIST_CONTEXT_PARTIAL_PREFIX: &str = "Watchlist context refresh partially failed:";
const WATCHLIST_HISTORY_FAILURE_PREFIX: &str = "Watchlist history refresh failed:";

pub(super) fn apply_contexts_loaded(
    loading: &mut bool,
    last_fetch_ms: &mut Option<u64>,
    contexts: &mut HashMap<String, WatchlistContext>,
    status: &mut Option<(String, bool)>,
    requested_at: u64,
    result: Result<WatchlistContextsResponse, String>,
) {
    *loading = false;

    match result {
        Ok(response) => {
            *last_fetch_ms = Some(requested_at);
            *contexts = response.contexts;
            if !response.partial_errors.is_empty() {
                *status = Some(live_watchlist_failure_status(
                    WATCHLIST_CONTEXT_PARTIAL_PREFIX,
                    &response.partial_errors.join("; "),
                ));
            } else if status.as_ref().is_some_and(|(message, _)| {
                message.starts_with(WATCHLIST_CONTEXT_FAILURE_PREFIX)
                    || message.starts_with(WATCHLIST_CONTEXT_PARTIAL_PREFIX)
            }) {
                *status = None;
            }
        }
        Err(error) => {
            *status = Some(live_watchlist_failure_status(
                WATCHLIST_CONTEXT_FAILURE_PREFIX,
                &error,
            ));
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

    match result {
        Ok(loaded_history) => {
            for symbol in requested_symbols {
                history.remove(&symbol);
                loaded_at.insert(symbol, requested_at);
            }
            history.extend(loaded_history);
        }
        Err(error) => {
            *status = Some(live_watchlist_failure_status(
                WATCHLIST_HISTORY_FAILURE_PREFIX,
                &error,
            ));
        }
    }
}

fn live_watchlist_failure_status(prefix: &str, error: &str) -> (String, bool) {
    (
        format!("{prefix} {}", redact_sensitive_response_text(error)),
        true,
    )
}
