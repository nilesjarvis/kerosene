use crate::api::WatchlistContext;

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Contexts
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

    match result {
        Ok(loaded_contexts) => {
            *last_fetch_ms = Some(requested_at);
            contexts.extend(loaded_contexts);
            *status = None;
        }
        Err(error) => {
            *status = Some((format!("24h volume refresh failed: {error}"), true));
        }
    }
}
