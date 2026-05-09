use crate::market_state::LiveWatchlistInstance;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Live Watchlist Symbols
// ---------------------------------------------------------------------------

pub(super) fn update_watchlist_search(watchlist: &mut LiveWatchlistInstance, query: String) {
    watchlist.search_query = query;
}

pub(super) fn add_watchlist_symbol(watchlist: &mut LiveWatchlistInstance, symbol: String) -> bool {
    if watchlist.symbols.contains(&symbol) {
        return false;
    }

    watchlist.symbols.push(symbol);
    watchlist.search_query.clear();
    true
}

pub(super) fn remove_watchlist_symbol(watchlist: &mut LiveWatchlistInstance, symbol: &str) {
    watchlist.symbols.retain(|candidate| candidate != symbol);
}
