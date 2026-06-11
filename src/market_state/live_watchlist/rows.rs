use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::positive_percent_change as percent_change;
use crate::market_state::{LiveWatchlistId, LiveWatchlistInstance, LiveWatchlistRowData};

use std::cmp::Ordering;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Live Watchlist Row Cache
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn refresh_live_watchlist_row_caches(&mut self) {
        let ids: Vec<LiveWatchlistId> = self.live_watchlists.keys().copied().collect();
        for id in ids {
            self.refresh_live_watchlist_row_cache(id);
        }
    }

    pub(crate) fn refresh_live_watchlist_row_cache(&mut self, id: LiveWatchlistId) {
        let Some(watchlist) = self.live_watchlists.get(&id) else {
            return;
        };
        let rows = self.live_watchlist_rows_for(watchlist);
        if let Some(watchlist) = self.live_watchlists.get_mut(&id) {
            watchlist.row_cache = rows;
        }
    }

    fn live_watchlist_rows_for(
        &self,
        watchlist: &LiveWatchlistInstance,
    ) -> Vec<LiveWatchlistRowData> {
        let symbols_by_key: HashMap<&str, &ExchangeSymbol> = self
            .exchange_symbols
            .iter()
            .map(|symbol| (symbol.key.as_str(), symbol))
            .collect();
        let mut rows = Vec::with_capacity(watchlist.symbols.len());

        for sym_key in &watchlist.symbols {
            if self.symbol_key_is_hidden(sym_key) {
                continue;
            }
            let sym_meta = symbols_by_key.get(sym_key.as_str()).copied();
            if sym_meta.is_some_and(|symbol| !symbol.is_user_selectable_market()) {
                continue;
            }
            let display = sym_meta
                .map(Self::exchange_symbol_display_name)
                .unwrap_or_else(|| self.display_name_for_symbol(sym_key));
            let mid_px = self.resolve_mid_for_symbol(sym_key);
            let ctx = self.live_watchlist_ctxs.get(sym_key).or_else(|| {
                sym_meta.and_then(|symbol| self.live_watchlist_ctxs.get(&symbol.ticker))
            });
            let prev_px = ctx.and_then(|ctx| ctx.prev_day_px);
            let funding = ctx.and_then(|ctx| ctx.funding);
            let (px_5m, px_30m, px_1h) = self
                .live_watchlist_history
                .get(sym_key)
                .copied()
                .map(|(px_5m, px_30m, px_1h)| (Some(px_5m), Some(px_30m), Some(px_1h)))
                .unwrap_or((None, None, None));

            rows.push(LiveWatchlistRowData {
                sym_key: sym_key.clone(),
                display,
                mid_px,
                pct_5m: percent_change(mid_px, px_5m),
                pct_30m: percent_change(mid_px, px_30m),
                pct_1h: percent_change(mid_px, px_1h),
                pct_24h: prev_px.and_then(|px| percent_change(mid_px, Some(px))),
                funding,
            });
        }

        sort_live_watchlist_rows(rows, watchlist.sort_column, watchlist.sort_direction)
    }
}

fn sort_live_watchlist_rows(
    mut rows: Vec<LiveWatchlistRowData>,
    sort_column: config::LiveWatchlistSortColumn,
    sort_direction: config::SortDirection,
) -> Vec<LiveWatchlistRowData> {
    let descending = sort_direction == config::SortDirection::Descending;
    rows.sort_by(|a, b| match sort_column {
        config::LiveWatchlistSortColumn::Symbol => {
            let cmp = a.display.cmp(&b.display);
            if descending { cmp.reverse() } else { cmp }
        }
        config::LiveWatchlistSortColumn::Price => sortable_cmp(a.mid_px, b.mid_px, descending),
        config::LiveWatchlistSortColumn::Change5m => sortable_cmp(a.pct_5m, b.pct_5m, descending),
        config::LiveWatchlistSortColumn::Change30m => {
            sortable_cmp(a.pct_30m, b.pct_30m, descending)
        }
        config::LiveWatchlistSortColumn::Change1h => sortable_cmp(a.pct_1h, b.pct_1h, descending),
        config::LiveWatchlistSortColumn::Change24h => {
            sortable_cmp(a.pct_24h, b.pct_24h, descending)
        }
        config::LiveWatchlistSortColumn::Funding => sortable_cmp(a.funding, b.funding, descending),
    });
    rows
}

fn sortable_cmp(a: Option<f64>, b: Option<f64>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            let cmp = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
            if descending { cmp.reverse() } else { cmp }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
