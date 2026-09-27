use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartBackfillFetchContext;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use iced::Task;
use std::collections::{HashMap, HashSet};

impl TradingTerminal {
    /// Rewrite persisted indexed aliases for API-named spot pairs (currently
    /// legacy `@0` -> `PURR/USDC`) once strict spot metadata proves the
    /// canonical key. Any request already issued with the old key is
    /// invalidated, and affected widgets are refetched under the canonical key.
    pub(super) fn migrate_legacy_spot_widget_keys(&mut self) -> Vec<Task<Message>> {
        let aliases: HashMap<String, (String, String)> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Spot)
            .filter_map(|symbol| {
                let spot_index = symbol.asset_index.checked_sub(10_000)?;
                let indexed_key = format!("@{spot_index}");
                (indexed_key != symbol.key).then(|| {
                    (
                        indexed_key,
                        (
                            symbol.key.clone(),
                            Self::exchange_symbol_display_name(symbol),
                        ),
                    )
                })
            })
            .collect();
        if aliases.is_empty() {
            return Vec::new();
        }

        let mut changed = false;
        let mut order_book_ids = Vec::new();
        for (id, instance) in &mut self.order_books {
            let OrderBookSymbolMode::Fixed(symbol) = &mut instance.mode else {
                continue;
            };
            let Some((canonical, _)) = aliases.get(symbol) else {
                continue;
            };
            *symbol = canonical.clone();
            instance.set_book(crate::api::OrderBook::empty());
            instance.clear_asset_context_and_price_history();
            instance.reset_tick_options_basis();
            instance.clear_book_request();
            instance.book_loading = true;
            instance.book_error = None;
            instance.book_failure_toasted = false;
            order_book_ids.push(*id);
            changed = true;
        }

        let chart_backfill_source = self.chart_backfill_source;
        let read_data_provider_generation = self.read_data_provider_generation;
        let hydromancer_key_generation = self.hydromancer_key_generation;
        let hydromancer_api_key = self.hydromancer_api_key_for_task();
        let now_ms = Self::now_ms();
        let mut removed_spaghetti_cache_keys = Vec::new();
        let mut spaghetti_fetches = Vec::new();
        for (chart_id, instance) in &mut self.spaghetti_charts {
            let effective_timeframe = Self::spaghetti_effective_timeframe_for(
                instance.interval,
                instance.canvas.active_session,
                instance.session_granularity,
                now_ms,
            );
            let mut chart_changed = false;
            for series in &mut instance.canvas.series {
                let Some((canonical, display)) = aliases.get(&series.symbol) else {
                    continue;
                };
                removed_spaghetti_cache_keys.push((series.symbol.clone(), effective_timeframe));
                series.symbol = canonical.clone();
                series.display = display.clone();
                chart_changed = true;
            }
            if !chart_changed {
                continue;
            }

            let mut seen = std::collections::HashSet::new();
            instance
                .canvas
                .series
                .retain(|series| seen.insert(series.symbol.clone()));
            for series in &mut instance.canvas.series {
                removed_spaghetti_cache_keys.push((series.symbol.clone(), effective_timeframe));
                series.candles.clear();
                series.loaded = false;
                spaghetti_fetches.push((
                    *chart_id,
                    series.symbol.clone(),
                    instance.interval,
                    instance.canvas.active_session,
                    instance.session_granularity,
                ));
            }
            instance.canvas.cache.clear();
            changed = true;
        }
        for (symbol, timeframe) in removed_spaghetti_cache_keys {
            self.remove_cached_candles(&symbol, timeframe);
        }

        let mut watchlist_changed = false;
        let mut legacy_watchlist_keys = HashSet::new();
        let mut changed_preset_ids = Vec::new();
        for preset in &mut self.watchlist_presets {
            if migrate_watchlist_symbols(&mut preset.symbols, &aliases, &mut legacy_watchlist_keys)
            {
                changed_preset_ids.push(preset.id);
                watchlist_changed = true;
            }
        }
        for preset_id in changed_preset_ids {
            self.sync_saved_layout_watchlist_preset_snapshots(preset_id);
        }
        for watchlist in self.live_watchlists.values_mut() {
            if migrate_watchlist_symbols(
                &mut watchlist.symbols,
                &aliases,
                &mut legacy_watchlist_keys,
            ) {
                watchlist_changed = true;
            }
        }
        if watchlist_changed {
            for legacy_key in legacy_watchlist_keys {
                self.live_watchlist_ctxs.remove(&legacy_key);
                self.live_watchlist_history.remove(&legacy_key);
                self.live_watchlist_history_loaded_at.remove(&legacy_key);
            }
            self.live_watchlist_contexts_request_id =
                self.live_watchlist_contexts_request_id.saturating_add(1);
            self.live_watchlist_contexts_loading = false;
            self.live_watchlist_contexts_request_symbols.clear();
            self.live_watchlist_contexts_refresh_pending = false;
            self.live_watchlist_history_request_id =
                self.live_watchlist_history_request_id.saturating_add(1);
            self.live_watchlist_history_loading = false;
            self.live_watchlist_history_request_symbols.clear();
            self.live_watchlist_history_refresh_pending = false;
            self.refresh_live_watchlist_row_caches();
            changed = true;
        }

        if !changed {
            return Vec::new();
        }
        self.persist_config();

        let spaghetti_instance_epoch = self.spaghetti_instance_epoch;
        let mut tasks = Vec::new();
        tasks.extend(
            order_book_ids
                .into_iter()
                .map(|id| self.order_book_fetch_task_for_id(id)),
        );
        tasks.extend(spaghetti_fetches.into_iter().map(
            |(chart_id, symbol, timeframe, session, session_granularity)| {
                Self::fetch_spaghetti_candles(
                    chart_id,
                    spaghetti_instance_epoch,
                    &symbol,
                    timeframe,
                    session,
                    session_granularity,
                    ChartBackfillFetchContext::new(
                        chart_backfill_source,
                        read_data_provider_generation,
                        hydromancer_key_generation,
                        hydromancer_api_key.clone(),
                    ),
                )
            },
        ));
        if watchlist_changed {
            tasks.push(self.request_live_watchlist_refresh(true));
        }
        tasks
    }
}

/// Rewrite aliases and deduplicate only lists that changed, retaining their order.
fn migrate_watchlist_symbols(
    symbols: &mut Vec<String>,
    aliases: &HashMap<String, (String, String)>,
    legacy_keys: &mut HashSet<String>,
) -> bool {
    let mut changed = false;
    for symbol in symbols.iter_mut() {
        if let Some((canonical, _)) = aliases.get(symbol) {
            legacy_keys.insert(std::mem::replace(symbol, canonical.clone()));
            changed = true;
        }
    }
    if changed {
        let mut seen = HashSet::new();
        symbols.retain(|symbol| seen.insert(symbol.clone()));
    }
    changed
}
