use crate::api::WatchlistContext;

use std::collections::HashMap;

use super::SLOW_DATA_REFRESH_MS;

// ---------------------------------------------------------------------------
// Context Refresh Decisions
// ---------------------------------------------------------------------------

pub(super) fn should_refresh_contexts(
    symbols: &[String],
    force: bool,
    now_ms: u64,
    contexts_last_fetch_ms: Option<u64>,
    contexts: &HashMap<String, WatchlistContext>,
    contexts_loading: bool,
) -> bool {
    if symbols.is_empty() || contexts_loading {
        return false;
    }

    let contexts_stale = contexts_last_fetch_ms
        .is_none_or(|last| now_ms.saturating_sub(last) >= SLOW_DATA_REFRESH_MS);
    let contexts_missing = symbols.iter().any(|symbol| !contexts.contains_key(symbol));

    force || contexts_stale || contexts_missing
}
