use crate::config::{LiveWatchlistColumn, LiveWatchlistSortColumn, SortDirection};
use crate::market_state::LiveWatchlistInstance;

use super::*;

fn watchlist() -> LiveWatchlistInstance {
    LiveWatchlistInstance {
        id: 11,
        symbols: vec!["BTC".to_string(), "ETH".to_string()],
        search_query: "sol".to_string(),
        sort_column: LiveWatchlistSortColumn::Symbol,
        sort_direction: SortDirection::Ascending,
        visible_columns: vec![LiveWatchlistColumn::Price],
        row_cache: Vec::new(),
    }
}

#[test]
fn search_query_update_replaces_previous_query() {
    let mut watchlist = watchlist();

    update_watchlist_search(&mut watchlist, "arb".to_string());

    assert_eq!(watchlist.search_query, "arb");
}

#[test]
fn add_symbol_appends_missing_symbol_and_clears_search() {
    let mut watchlist = watchlist();

    let added = add_watchlist_symbol(&mut watchlist, "SOL".to_string());

    assert!(added);
    assert_eq!(
        watchlist.symbols,
        vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()]
    );
    assert_eq!(watchlist.search_query, "");
}

#[test]
fn add_symbol_rejects_duplicate_and_keeps_search() {
    let mut watchlist = watchlist();

    let added = add_watchlist_symbol(&mut watchlist, "ETH".to_string());

    assert!(!added);
    assert_eq!(
        watchlist.symbols,
        vec!["BTC".to_string(), "ETH".to_string()]
    );
    assert_eq!(watchlist.search_query, "sol");
}

#[test]
fn remove_symbol_drops_all_matching_entries() {
    let mut watchlist = watchlist();
    watchlist.symbols.push("ETH".to_string());

    remove_watchlist_symbol(&mut watchlist, "ETH");

    assert_eq!(watchlist.symbols, vec!["BTC".to_string()]);
}
