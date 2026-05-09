use crate::config::{LiveWatchlistColumn, LiveWatchlistSortColumn, SortDirection};
use crate::market_state::LiveWatchlistInstance;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Live Watchlist Controls
// ---------------------------------------------------------------------------

pub(super) fn apply_sort_change(
    watchlist: &mut LiveWatchlistInstance,
    column: LiveWatchlistSortColumn,
) {
    if watchlist.sort_column == column {
        watchlist.sort_direction = match watchlist.sort_direction {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        };
    } else {
        watchlist.sort_column = column;
        watchlist.sort_direction = SortDirection::Descending;
    }
}

pub(super) fn apply_column_toggle(
    watchlist: &mut LiveWatchlistInstance,
    column: LiveWatchlistColumn,
    enabled: bool,
) {
    if enabled {
        if !watchlist.visible_columns.contains(&column) {
            watchlist.visible_columns.push(column);
            sort_visible_columns(&mut watchlist.visible_columns);
        }
    } else {
        watchlist
            .visible_columns
            .retain(|candidate| candidate != &column);
        if watchlist.sort_column == column.sort_column() {
            watchlist.sort_column = LiveWatchlistSortColumn::Symbol;
            watchlist.sort_direction = SortDirection::Ascending;
        }
    }
}

fn sort_visible_columns(columns: &mut [LiveWatchlistColumn]) {
    columns.sort_by_key(|candidate| {
        LiveWatchlistColumn::ALL
            .iter()
            .position(|known| known == candidate)
            .unwrap_or(usize::MAX)
    });
}
