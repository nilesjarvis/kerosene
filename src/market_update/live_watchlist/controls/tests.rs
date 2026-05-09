use crate::config::{LiveWatchlistColumn, LiveWatchlistSortColumn, SortDirection};
use crate::market_state::LiveWatchlistInstance;

use super::*;

fn watchlist() -> LiveWatchlistInstance {
    LiveWatchlistInstance {
        id: 7,
        symbols: Vec::new(),
        search_query: String::new(),
        sort_column: LiveWatchlistSortColumn::Symbol,
        sort_direction: SortDirection::Ascending,
        visible_columns: vec![LiveWatchlistColumn::Price, LiveWatchlistColumn::Funding],
        row_cache: Vec::new(),
    }
}

#[test]
fn sort_change_toggles_existing_column_direction() {
    let mut watchlist = watchlist();

    apply_sort_change(&mut watchlist, LiveWatchlistSortColumn::Symbol);
    assert_eq!(watchlist.sort_column, LiveWatchlistSortColumn::Symbol);
    assert_eq!(watchlist.sort_direction, SortDirection::Descending);

    apply_sort_change(&mut watchlist, LiveWatchlistSortColumn::Symbol);
    assert_eq!(watchlist.sort_direction, SortDirection::Ascending);
}

#[test]
fn sort_change_new_column_defaults_to_descending() {
    let mut watchlist = watchlist();

    apply_sort_change(&mut watchlist, LiveWatchlistSortColumn::Change24h);

    assert_eq!(watchlist.sort_column, LiveWatchlistSortColumn::Change24h);
    assert_eq!(watchlist.sort_direction, SortDirection::Descending);
}

#[test]
fn enabling_column_restores_known_order() {
    let mut watchlist = watchlist();
    watchlist.visible_columns = vec![LiveWatchlistColumn::Funding, LiveWatchlistColumn::Price];

    apply_column_toggle(&mut watchlist, LiveWatchlistColumn::Change5m, true);

    assert_eq!(
        watchlist.visible_columns,
        vec![
            LiveWatchlistColumn::Price,
            LiveWatchlistColumn::Change5m,
            LiveWatchlistColumn::Funding,
        ]
    );
}

#[test]
fn enabling_existing_column_does_not_duplicate() {
    let mut watchlist = watchlist();

    apply_column_toggle(&mut watchlist, LiveWatchlistColumn::Price, true);

    assert_eq!(
        watchlist.visible_columns,
        vec![LiveWatchlistColumn::Price, LiveWatchlistColumn::Funding]
    );
}

#[test]
fn disabling_sorted_column_resets_sort_to_symbol_ascending() {
    let mut watchlist = watchlist();
    watchlist.sort_column = LiveWatchlistSortColumn::Funding;
    watchlist.sort_direction = SortDirection::Descending;

    apply_column_toggle(&mut watchlist, LiveWatchlistColumn::Funding, false);

    assert_eq!(watchlist.visible_columns, vec![LiveWatchlistColumn::Price]);
    assert_eq!(watchlist.sort_column, LiveWatchlistSortColumn::Symbol);
    assert_eq!(watchlist.sort_direction, SortDirection::Ascending);
}

#[test]
fn disabling_unsorted_column_keeps_current_sort() {
    let mut watchlist = watchlist();
    watchlist.sort_column = LiveWatchlistSortColumn::Price;
    watchlist.sort_direction = SortDirection::Descending;

    apply_column_toggle(&mut watchlist, LiveWatchlistColumn::Funding, false);

    assert_eq!(watchlist.visible_columns, vec![LiveWatchlistColumn::Price]);
    assert_eq!(watchlist.sort_column, LiveWatchlistSortColumn::Price);
    assert_eq!(watchlist.sort_direction, SortDirection::Descending);
}
