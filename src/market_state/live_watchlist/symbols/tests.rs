use crate::config::{LiveWatchlistColumn, LiveWatchlistSortColumn, SortDirection};

use super::*;

fn watchlist(id: LiveWatchlistId, symbols: &[&str]) -> LiveWatchlistInstance {
    LiveWatchlistInstance {
        id,
        symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
        search_query: String::new(),
        sort_column: LiveWatchlistSortColumn::Symbol,
        sort_direction: SortDirection::Ascending,
        visible_columns: vec![LiveWatchlistColumn::Price],
        row_cache: Vec::new(),
    }
}

#[test]
fn open_ids_collect_only_live_watchlist_panes() {
    let panes = vec![
        PaneKind::Watchlist,
        PaneKind::LiveWatchlist(2),
        PaneKind::OrderBook(5),
        PaneKind::LiveWatchlist(7),
    ];

    let ids = open_live_watchlist_ids(&panes);

    assert_eq!(ids, HashSet::from([2, 7]));
}

#[test]
fn watched_symbols_use_open_watchlists_filter_muted_and_deduplicate() {
    let live_watchlists = HashMap::from([
        (1, watchlist(1, &["SOL", "ETH"])),
        (2, watchlist(2, &["BTC", "ETH"])),
        (3, watchlist(3, &["DOGE"])),
    ]);
    let open_ids = HashSet::from([1, 2]);

    let symbols = watched_symbol_keys(&live_watchlists, &open_ids, |symbol| symbol == "SOL");

    assert_eq!(symbols, vec!["BTC".to_string(), "ETH".to_string()]);
}

#[test]
fn watched_symbols_empty_when_no_watchlist_is_open() {
    let live_watchlists = HashMap::from([(1, watchlist(1, &["BTC"]))]);

    let symbols = watched_symbol_keys(&live_watchlists, &HashSet::new(), |_| false);

    assert!(symbols.is_empty());
}
