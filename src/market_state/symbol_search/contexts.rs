use super::SymbolSearchSortMode;
use crate::api::WatchlistContext;

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Context Refresh
// ---------------------------------------------------------------------------

const SYMBOL_SEARCH_CONTEXT_REFRESH_MS: u64 = 300_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SymbolSearchContextRefreshPlan {
    pub(super) requested_at: u64,
    pub(super) symbols: Vec<String>,
}

pub(super) struct SymbolSearchContextRefreshInput<'a> {
    pub(super) symbols: Vec<String>,
    pub(super) force: bool,
    pub(super) now_ms: u64,
    pub(super) sort_mode: SymbolSearchSortMode,
    pub(super) contexts_loading: bool,
    pub(super) contexts_last_fetch_ms: Option<u64>,
    pub(super) contexts: &'a HashMap<String, WatchlistContext>,
}

pub(super) fn plan_context_refresh(
    input: SymbolSearchContextRefreshInput<'_>,
) -> Option<SymbolSearchContextRefreshPlan> {
    if input.sort_mode != SymbolSearchSortMode::Volume24h
        || input.contexts_loading
        || input.symbols.is_empty()
    {
        return None;
    }

    let contexts_stale = input
        .contexts_last_fetch_ms
        .is_none_or(|last| input.now_ms.saturating_sub(last) >= SYMBOL_SEARCH_CONTEXT_REFRESH_MS);
    let contexts_missing = input
        .symbols
        .iter()
        .any(|symbol| !input.contexts.contains_key(symbol));

    if !input.force && !contexts_stale && !contexts_missing {
        return None;
    }

    Some(SymbolSearchContextRefreshPlan {
        requested_at: input.now_ms,
        symbols: input.symbols,
    })
}
