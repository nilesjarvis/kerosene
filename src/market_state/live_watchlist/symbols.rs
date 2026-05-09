use super::LiveWatchlistId;
use crate::market_state::LiveWatchlistInstance;
use crate::pane_state::PaneKind;

use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Live Watchlist Symbols
// ---------------------------------------------------------------------------

pub(super) fn open_live_watchlist_ids<'a, I>(panes: I) -> HashSet<LiveWatchlistId>
where
    I: IntoIterator<Item = &'a PaneKind>,
{
    panes
        .into_iter()
        .filter_map(|kind| match kind {
            PaneKind::LiveWatchlist(id) => Some(*id),
            _ => None,
        })
        .collect()
}

pub(super) fn watched_symbol_keys<IsMuted>(
    live_watchlists: &HashMap<LiveWatchlistId, LiveWatchlistInstance>,
    open_ids: &HashSet<LiveWatchlistId>,
    mut is_muted: IsMuted,
) -> Vec<String>
where
    IsMuted: FnMut(&str) -> bool,
{
    let mut symbols: Vec<String> = live_watchlists
        .iter()
        .filter(|(id, _)| open_ids.contains(id))
        .flat_map(|(_, watchlist)| watchlist.symbols.iter().cloned())
        .filter(|symbol| !is_muted(symbol))
        .collect();

    symbols.sort();
    symbols.dedup();
    symbols
}
