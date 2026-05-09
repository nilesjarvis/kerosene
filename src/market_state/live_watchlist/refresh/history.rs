use std::collections::HashMap;

use super::SLOW_DATA_REFRESH_MS;

// ---------------------------------------------------------------------------
// History Refresh Decisions
// ---------------------------------------------------------------------------

pub(super) fn select_history_symbols(
    symbols: Vec<String>,
    force: bool,
    now_ms: u64,
    history_loaded_at: &HashMap<String, u64>,
    history_loading: bool,
) -> Vec<String> {
    if history_loading {
        return Vec::new();
    }

    symbols
        .into_iter()
        .filter(|symbol| {
            force
                || history_loaded_at
                    .get(symbol)
                    .is_none_or(|last| now_ms.saturating_sub(*last) >= SLOW_DATA_REFRESH_MS)
        })
        .collect()
}
