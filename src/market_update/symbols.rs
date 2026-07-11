mod contexts;
mod controls;
mod outcome_volumes;
mod resolution;

use crate::api::{ExchangeSymbolsPayload, MarketType};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartBackfillFetchContext;
use crate::config::MarketUniverseConfig;
use crate::helpers::redact_sensitive_response_text;
use crate::market_state::{OrderBookSymbolMode, SymbolSearchMarketFilter};
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;

use self::contexts::apply_contexts_loaded;
use self::controls::{apply_hip3_dex_filter, apply_market_filter, toggle_favourite_symbol};
use resolution::resolve_exchange_symbol;

use iced::Task;

impl TradingTerminal {
    pub(super) fn update_symbol_search_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleFavourite(key) => self.toggle_market_favourite(key),
            Message::SymbolsLoaded(request_id, result) => {
                self.apply_symbols_loaded_message(request_id, result.into_result())
            }
            Message::ExchangeSymbolsRefreshTick => self.request_exchange_symbols_refresh(),
            Message::SymbolSearchChanged(query) => {
                self.symbol_search_query = query;
                self.refresh_symbol_search_results();
                Task::none()
            }
            Message::SymbolSearchSortChanged(sort_mode) => {
                self.symbol_search_sort_mode = sort_mode;
                self.refresh_symbol_search_results();
                self.persist_config();
                self.request_symbol_search_context_refresh(false)
            }
            Message::SymbolSearchMarketFilterChanged(filter) => {
                apply_market_filter(
                    &mut self.symbol_search_market_filter,
                    &mut self.symbol_search_hip3_dex_filter,
                    filter,
                );
                self.refresh_symbol_search_results();
                self.request_symbol_search_context_refresh(false)
            }
            Message::SymbolSearchHip3DexFilterChanged(dex) => {
                apply_hip3_dex_filter(&mut self.symbol_search_hip3_dex_filter, dex);
                self.refresh_symbol_search_results();
                self.request_symbol_search_context_refresh(false)
            }
            Message::SymbolSearchContextsLoaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ) => self.apply_symbol_search_contexts_loaded(
                request_id,
                requested_symbols,
                requested_at,
                result.into_result(),
            ),
            Message::OutcomeSearchChanged(query) => {
                self.outcome_search_query = query;
                Task::none()
            }
            Message::OutcomeMarketGroupToggled(key) => {
                if !self.outcome_collapsed_market_groups.insert(key.clone()) {
                    self.outcome_collapsed_market_groups.remove(&key);
                }
                Task::none()
            }
            Message::OutcomeVolumesLoaded(request_id, requested_symbols, result) => self
                .apply_outcome_volumes_loaded(request_id, requested_symbols, result.into_result()),
            Message::SymbolSelected(key) => self.select_market_symbol(key),
            _ => Task::none(),
        }
    }

    fn toggle_market_favourite(&mut self, key: String) -> Task<Message> {
        if self.symbol_key_is_hidden(&key) {
            let display = self.display_name_for_symbol(&key);
            self.symbol_search_status =
                Some((format!("{display} is hidden by Settings > Risk"), true));
            return Task::none();
        }
        toggle_favourite_symbol(&mut self.favourite_symbols, key);
        self.refresh_symbol_search_results();
        self.persist_config();
        self.request_ticker_tape_context_refresh(true)
    }

    fn request_exchange_symbols_refresh(&mut self) -> Task<Message> {
        if self.symbols_loading || self.exchange_symbols_refresh_inflight {
            return Task::none();
        }

        self.request_live_exchange_symbols()
    }

    fn advance_exchange_symbols_request_id(&mut self) -> u64 {
        // Advancing again when a completion is accepted independently rejects
        // duplicate delivery. Wrapping keeps the next generation distinct from
        // its immediate predecessor even at the counter boundary.
        self.exchange_symbols_request_id = self.exchange_symbols_request_id.wrapping_add(1);
        self.exchange_symbols_request_id
    }

    fn request_live_exchange_symbols(&mut self) -> Task<Message> {
        self.exchange_symbols_refresh_inflight = true;
        let request_id = self.advance_exchange_symbols_request_id();
        Task::perform(crate::api::fetch_exchange_symbols(), move |result| {
            Message::SymbolsLoaded(request_id, result.into())
        })
    }

    /// A failed metadata request leaves that market type absent from the
    /// payload. Retained spot symbols stay visible but are fail-closed for new
    /// orders; retained outcome symbols are label-only until fresh metadata
    /// proves either market type orderable again.
    fn merge_symbols_payload(
        &self,
        payload: ExchangeSymbolsPayload,
    ) -> Vec<crate::api::ExchangeSymbol> {
        let ExchangeSymbolsPayload {
            mut symbols,
            loaded_from_cache: _,
            perp_meta_failed,
            spot_meta_failed,
            outcome_meta_failed,
        } = payload;

        if perp_meta_failed {
            symbols.extend(
                self.exchange_symbols
                    .iter()
                    .filter(|symbol| symbol.market_type == MarketType::Perp)
                    .cloned(),
            );
        }
        if spot_meta_failed {
            symbols.extend(
                self.exchange_symbols
                    .iter()
                    .filter(|symbol| symbol.market_type == MarketType::Spot)
                    .cloned(),
            );
        }
        if outcome_meta_failed {
            symbols.extend(
                self.exchange_symbols
                    .iter()
                    .filter(|symbol| symbol.market_type == MarketType::Outcome)
                    .cloned()
                    .map(|mut symbol| {
                        symbol.display_name = Some(
                            self.outcome_display_labels
                                .get(&symbol.key)
                                .cloned()
                                .unwrap_or_else(|| Self::exchange_symbol_display_name(&symbol)),
                        );
                        symbol.outcome = None;
                        symbol
                    }),
            );
        }
        if perp_meta_failed || spot_meta_failed || outcome_meta_failed {
            symbols.sort_by(|a, b| a.ticker.cmp(&b.ticker));
        }
        symbols
    }

    /// Rewrite persisted indexed aliases for API-named spot pairs (currently
    /// legacy `@0` -> `PURR/USDC`) once strict spot metadata proves the
    /// canonical key. Any request already issued with the old key is
    /// invalidated, and affected widgets are refetched under the canonical key.
    fn migrate_legacy_spot_widget_keys(&mut self) -> Vec<Task<Message>> {
        let aliases: std::collections::HashMap<String, (String, String)> = self
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
        let chart_instance_generation = self.chart_instance_generation;
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
            instance.clear_spaghetti_candle_requests();
            let symbols = instance
                .canvas
                .series
                .iter_mut()
                .map(|series| {
                    removed_spaghetti_cache_keys.push((series.symbol.clone(), effective_timeframe));
                    series.candles.clear();
                    series.loaded = false;
                    series.symbol.clone()
                })
                .collect::<Vec<_>>();
            for symbol in symbols {
                spaghetti_fetches.push((*chart_id, symbol));
            }
            instance.canvas.cache.clear();
            changed = true;
        }
        for (symbol, timeframe) in removed_spaghetti_cache_keys {
            self.remove_cached_candles(&symbol, timeframe);
        }

        let mut watchlist_changed = false;
        let mut legacy_watchlist_keys = std::collections::HashSet::new();
        for watchlist in self.live_watchlists.values_mut() {
            let mut this_watchlist_changed = false;
            for symbol in &mut watchlist.symbols {
                let Some((canonical, _)) = aliases.get(symbol) else {
                    continue;
                };
                legacy_watchlist_keys.insert(symbol.clone());
                *symbol = canonical.clone();
                this_watchlist_changed = true;
                watchlist_changed = true;
            }
            if this_watchlist_changed {
                let mut seen = std::collections::HashSet::new();
                watchlist
                    .symbols
                    .retain(|symbol| seen.insert(symbol.clone()));
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

        let mut tasks = Vec::new();
        tasks.extend(
            order_book_ids
                .into_iter()
                .map(|id| self.order_book_fetch_task_for_id(id)),
        );
        for (chart_id, symbol) in spaghetti_fetches {
            if let Some(instance) = self.spaghetti_charts.get_mut(&chart_id) {
                tasks.push(Self::queue_spaghetti_candle_fetch(
                    instance,
                    &symbol,
                    chart_instance_generation,
                    None,
                    ChartBackfillFetchContext::new(
                        chart_backfill_source,
                        read_data_provider_generation,
                        hydromancer_key_generation,
                        hydromancer_api_key.clone(),
                    ),
                ));
            }
        }
        if watchlist_changed {
            tasks.push(self.request_live_watchlist_refresh(true));
        }
        tasks
    }

    /// Remember the display label of every loaded outcome market so fills,
    /// journal entries, and balances keep their names after the market
    /// expires and disappears from outcomeMeta.
    fn record_outcome_display_labels(&mut self) {
        let labels: Vec<(String, String)> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Outcome)
            .map(|symbol| {
                (
                    symbol.key.clone(),
                    Self::exchange_symbol_display_name(symbol),
                )
            })
            .collect();

        let mut changed = false;
        for (key, label) in labels {
            if self.outcome_display_labels.get(&key) != Some(&label) {
                self.outcome_display_labels.insert(key, label);
                changed = true;
            }
        }
        if changed {
            self.persist_config();
        }
    }

    /// Re-resolve cached spaghetti series labels after a symbols load so
    /// series restored before symbols arrived (or naming newly listed
    /// markets) pick up their proper display names.
    fn refresh_spaghetti_series_displays(&mut self) {
        let updates: Vec<(SpaghettiChartId, usize, String)> = self
            .spaghetti_charts
            .iter()
            .flat_map(|(id, inst)| {
                inst.canvas
                    .series
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, series)| {
                        let display = self.display_name_for_symbol(&series.symbol);
                        (display != series.display).then_some((*id, idx, display))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        for (id, idx, display) in updates {
            if let Some(inst) = self.spaghetti_charts.get_mut(&id)
                && let Some(series) = inst.canvas.series.get_mut(idx)
            {
                series.display = display;
                inst.canvas.cache.clear();
            }
        }
    }

    fn apply_symbols_loaded(
        &mut self,
        result: Result<ExchangeSymbolsPayload, String>,
    ) -> Task<Message> {
        self.exchange_symbols_refresh_inflight = false;
        match result {
            Ok(payload) => {
                let previous_user_data_dexes = self.visible_mids_dexes();
                let loaded_from_cache = payload.loaded_from_cache;
                let perp_meta_failed = payload.perp_meta_failed;
                let spot_meta_failed = payload.spot_meta_failed;
                let outcome_meta_failed = payload.outcome_meta_failed;
                let was_spot_metadata_degraded = self.spot_metadata_degraded;
                self.spot_metadata_degraded = spot_meta_failed || loaded_from_cache;
                let symbols = self.merge_symbols_payload(payload);
                let symbols_changed = self.exchange_symbols != symbols;
                if symbols_changed {
                    self.exchange_symbols = symbols;
                }
                let mut initial_tasks = if spot_meta_failed {
                    Vec::new()
                } else {
                    self.migrate_legacy_spot_widget_keys()
                };
                if loaded_from_cache {
                    self.symbol_search_status = Some((
                        "Cached markets are visible while live spot metadata is verified; spot trading remains disabled until verification succeeds"
                            .to_string(),
                        true,
                    ));
                    initial_tasks.push(self.request_live_exchange_symbols());
                } else if spot_meta_failed {
                    let retained = self
                        .exchange_symbols
                        .iter()
                        .any(|symbol| symbol.market_type == MarketType::Spot);
                    let message = if retained {
                        "Spot metadata is temporarily unverified; last-known spot markets remain visible, but spot trading is disabled until verification succeeds"
                    } else {
                        "Spot metadata failed to load; spot trading is unavailable until verification succeeds"
                    }
                    .to_string();
                    self.symbol_search_status = Some((message.clone(), true));
                    if !was_spot_metadata_degraded {
                        self.push_toast(message, true);
                    }
                } else if was_spot_metadata_degraded {
                    self.symbol_search_status = Some((
                        "Spot metadata verified; spot trading is available again".to_string(),
                        false,
                    ));
                } else if perp_meta_failed {
                    self.symbol_search_status = Some((
                        "Perpetual metadata failed to load; using last-known perpetual markets and retrying shortly"
                            .to_string(),
                        true,
                    ));
                } else if outcome_meta_failed
                    && !self.exchange_symbols.iter().any(|symbol| {
                        symbol.market_type == MarketType::Outcome && symbol.outcome.is_some()
                    })
                {
                    self.symbol_search_status = Some((
                        "Outcome market metadata failed to load; retrying shortly".to_string(),
                        true,
                    ));
                }
                if !symbols_changed && !self.exchange_symbols.is_empty() {
                    self.symbols_loading = false;
                    return Task::batch(initial_tasks);
                }
                self.record_outcome_display_labels();
                self.telegram_feed
                    .rebuild_ticker_mention_resolver(&self.exchange_symbols);
                self.refresh_telegram_ticker_mentions();
                let mut market_universe_changed = false;
                let normalized_universe =
                    self.normalize_market_universe_selection(self.market_universe.clone());
                if normalized_universe != self.market_universe {
                    self.market_universe = normalized_universe;
                    self.clear_percentage_order_quantity();
                    market_universe_changed = true;
                    self.symbol_search_status = Some((
                        "Saved market universe was unavailable; showing all markets".to_string(),
                        true,
                    ));
                    self.push_toast(
                        "Saved market universe was unavailable; showing all markets".to_string(),
                        true,
                    );
                    self.persist_config();
                }
                if self.visible_mids_dexes() != previous_user_data_dexes {
                    self.rotate_all_user_data_streams();
                }
                match self.market_universe.selected_hip3_dex() {
                    Some(dex) => {
                        self.symbol_search_market_filter = SymbolSearchMarketFilter::Hip3;
                        self.symbol_search_hip3_dex_filter = Some(dex.to_string());
                    }
                    None if matches!(self.market_universe, MarketUniverseConfig::All) => {
                        self.symbol_search_market_filter = SymbolSearchMarketFilter::All;
                        self.symbol_search_hip3_dex_filter = None;
                    }
                    None => {}
                }
                self.refresh_symbol_search_results();
                self.symbols_loading = false;

                let mut tasks = initial_tasks;
                tasks.extend(self.mids_bootstrap_tasks());

                let active_symbol = self.active_symbol.clone();
                let active_source_unavailable = (spot_meta_failed
                    && (active_symbol.starts_with('@') || active_symbol.contains('/')))
                    || (outcome_meta_failed && active_symbol.starts_with('#'))
                    || (perp_meta_failed
                        && !active_symbol.starts_with('@')
                        && !active_symbol.starts_with('#')
                        && !active_symbol.contains('/'));
                match (!active_source_unavailable)
                    .then(|| self.restored_active_symbol_key(&active_symbol))
                    .flatten()
                {
                    Some(valid_key) if valid_key != self.active_symbol => {
                        tasks.push(self.switch_active_symbol_internal(valid_key));
                    }
                    Some(valid_key) => {
                        if let Some(symbol) =
                            resolve_exchange_symbol(&self.exchange_symbols, &valid_key)
                        {
                            self.active_symbol_display = Self::exchange_symbol_display_name(symbol);
                        }
                        self.sync_order_leverage_form_for_active_symbol();
                    }
                    None => {
                        if !active_source_unavailable {
                            self.apply_active_symbol_selection(String::new(), String::new());
                            self.order_status =
                                Some(("No tradable market symbols are available".into(), true));
                        }
                    }
                }

                let chart_backfill_request_context = self.chart_backfill_request_context();
                let hydromancer_api_key = self.hydromancer_api_key_for_task();
                let schwab_access_token = self.schwab.access_token_for_task();
                let mut reset_quick_order_chart_ids = Vec::new();
                let mut chart_identity_changed = false;
                for (id, inst) in self.charts.iter_mut() {
                    let key = inst.symbol.clone();
                    let symbol = resolve_exchange_symbol(&self.exchange_symbols, &key);
                    let mut primary_alias_canonicalized = false;

                    if let Some(valid) = symbol {
                        let display = Self::exchange_symbol_display_name(valid);
                        let symbol_changed = valid.key != inst.symbol;
                        primary_alias_canonicalized = symbol_changed;

                        if symbol_changed || inst.symbol_display != display {
                            inst.set_symbol_identity(valid.key.clone(), display);
                        }

                        if symbol_changed {
                            chart_identity_changed = true;
                            inst.reset_quick_order_for_account_reset();
                            reset_quick_order_chart_ids.push(*id);
                            inst.chart.status = ChartStatus::Loading;
                            inst.chart.candles.clear();
                            inst.chart.clear_macro_candles();
                            inst.chart.candle_cache.clear();
                            inst.set_asset_context(None);
                            inst.candle_fetch_error = None;
                            Self::clear_chart_symbol_display_state(inst);
                            let request = Self::build_candle_fetch_request(
                                *id,
                                &valid.key,
                                inst.interval,
                                chart_backfill_request_context,
                                None,
                                0,
                            );
                            inst.candle_fetch_request = Some(request.clone());
                            let mut chart_tasks = vec![Self::fetch_candles_task(
                                request,
                                hydromancer_api_key.clone(),
                                schwab_access_token.clone(),
                            )];
                            let macro_request_id = inst.next_macro_candles_request_id();
                            chart_tasks.extend(Self::fetch_macro_candles_tasks(
                                *id,
                                chart_backfill_request_context.chart_instance_generation,
                                macro_request_id,
                                &valid.key,
                            ));
                            tasks.push(Task::batch(chart_tasks));
                        }
                    }
                    if let Some(secondary_key) = inst.secondary_symbol.clone()
                        && let Some(valid) =
                            resolve_exchange_symbol(&self.exchange_symbols, &secondary_key)
                    {
                        let display = Self::exchange_symbol_display_name(valid);
                        let symbol_changed =
                            inst.secondary_symbol.as_deref() != Some(valid.key.as_str());
                        let alias_collision = valid.key == inst.symbol
                            && (primary_alias_canonicalized || symbol_changed);

                        if alias_collision {
                            inst.clear_secondary_symbol();
                            chart_identity_changed = true;
                            continue;
                        }

                        if symbol_changed
                            || inst.secondary_symbol_display.as_deref() != Some(display.as_str())
                        {
                            inst.set_secondary_symbol_identity(valid.key.clone(), display);
                        }

                        if symbol_changed {
                            chart_identity_changed = true;
                            inst.chart.set_secondary_candles(Vec::new());
                            inst.secondary_candle_fetch_error = None;
                            let request = Self::build_candle_fetch_request(
                                *id,
                                &valid.key,
                                inst.interval,
                                chart_backfill_request_context,
                                None,
                                0,
                            );
                            inst.secondary_candle_fetch_request = Some(request.clone());
                            tasks.push(Self::fetch_secondary_candles_task(
                                request,
                                hydromancer_api_key.clone(),
                                schwab_access_token.clone(),
                            ));
                        }
                    }
                }
                for chart_id in reset_quick_order_chart_ids {
                    self.chart_quick_order_surface.remove(&chart_id);
                }
                if chart_identity_changed {
                    self.persist_config();
                }

                self.refresh_spaghetti_series_displays();
                tasks.push(self.reconcile_session_data_symbols());
                tasks.push(self.refresh_enabled_earnings_charts());
                tasks.push(self.scrub_hidden_symbol_state());
                self.refresh_symbol_search_results();
                self.refresh_live_watchlist_row_caches();
                tasks.push(self.request_symbol_search_context_refresh(false));
                tasks.push(self.request_ticker_tape_context_refresh(true));
                tasks.push(self.request_outcome_volume_refresh());
                tasks.push(self.request_screener_data_refresh(true));
                if market_universe_changed {
                    tasks.push(self.refresh_account_data());
                }

                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }
            Err(error) => {
                self.symbols_loading = false;
                // Background refreshes fail quietly; the next tick retries.
                if self.exchange_symbols.is_empty() {
                    let message = format!(
                        "Symbol load failed: {}",
                        redact_sensitive_response_text(&error)
                    );
                    self.symbol_search_status = Some((message.clone(), true));
                    self.push_toast(message, true);
                }
            }
        }

        Task::none()
    }

    fn apply_symbols_loaded_message(
        &mut self,
        request_id: u64,
        result: Result<ExchangeSymbolsPayload, String>,
    ) -> Task<Message> {
        if request_id != self.exchange_symbols_request_id
            || (!self.symbols_loading && !self.exchange_symbols_refresh_inflight)
        {
            return Task::none();
        }

        self.advance_exchange_symbols_request_id();
        self.apply_symbols_loaded(result)
    }

    fn select_market_symbol(&mut self, key: String) -> Task<Message> {
        if self.active_symbol == key {
            return Task::none();
        }

        self.switch_active_symbol_internal(key)
    }

    fn apply_symbol_search_contexts_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        requested_at: u64,
        result: Result<crate::api::WatchlistContextsResponse, String>,
    ) -> Task<Message> {
        if !self.symbol_search_contexts_loading
            || request_id != self.symbol_search_contexts_request_id
            || requested_symbols != self.symbol_search_contexts_request_symbols
        {
            return Task::none();
        }

        self.symbol_search_contexts_request_id =
            self.symbol_search_contexts_request_id.saturating_add(1);
        let refresh_pending = self.symbol_search_contexts_refresh_pending;
        self.symbol_search_contexts_refresh_pending = false;
        self.symbol_search_contexts_request_symbols.clear();

        let existing_contexts = self.symbol_search_ctxs.clone();
        let result = result.map(|mut response| {
            let requested_symbols: std::collections::HashSet<String> =
                requested_symbols.into_iter().collect();
            if !response.partial_errors.is_empty() {
                let mut merged = existing_contexts
                    .into_iter()
                    .filter(|(symbol, _)| requested_symbols.contains(symbol))
                    .collect::<std::collections::HashMap<_, _>>();
                merged.extend(response.contexts);
                response.contexts = merged;
            }
            response
                .contexts
                .retain(|symbol, _| requested_symbols.contains(symbol));
            response
        });

        apply_contexts_loaded(
            &mut self.symbol_search_contexts_loading,
            &mut self.symbol_search_contexts_last_fetch_ms,
            &mut self.symbol_search_ctxs,
            &mut self.symbol_search_status,
            requested_at,
            result,
        );
        self.refresh_symbol_search_results();

        if refresh_pending {
            return self.request_symbol_search_context_refresh(true);
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Candle, ExchangeSymbol, MarketType, OutcomeSymbolInfo, WatchlistContext};
    use crate::chart_state::{ChartInstance, ChartSurfaceId};
    use crate::hydromancer_api::FundingRatePoint;
    use crate::hyperdash_api::{
        HeatmapFetchParams, LiquidationBucket, LiquidationHeatmap, LiquidationLevel,
    };
    use crate::market_state::{
        LiveWatchlistInstance, OrderBookInstance, OrderBookSymbolMode, SymbolSearchMarketFilter,
        SymbolSearchSortMode,
    };
    use crate::message::Message;
    use crate::order_execution::QuickOrderForm;
    use crate::spaghetti::Series;
    use crate::spaghetti_state::SpaghettiChartInstance;
    use crate::timeframe::Timeframe;
    use std::collections::HashMap;

    fn perp_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: Some(0),
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn spot_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: "HYPE".to_string(),
            category: "spot".to_string(),
            display_name: Some("HYPE/USDC".to_string()),
            keywords: vec!["spot".to_string()],
            asset_index: 10_107,
            collateral_token: Some(0),
            sz_decimals: 2,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Spot,
            outcome: None,
        }
    }

    fn canonical_purr_symbol() -> ExchangeSymbol {
        ExchangeSymbol {
            key: "PURR/USDC".to_string(),
            ticker: "PURR".to_string(),
            category: "spot".to_string(),
            display_name: Some("PURR/USDC".to_string()),
            keywords: vec!["spot".to_string()],
            asset_index: 10_000,
            collateral_token: Some(0),
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Spot,
            outcome: None,
        }
    }

    fn outcome_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "outcome".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: true,
            market_type: MarketType::Outcome,
            outcome: Some(OutcomeSymbolInfo {
                outcome_id: 95,
                question_id: None,
                question_name: Some("Will BTC close green?".to_string()),
                question_description: None,
                question_class: None,
                question_underlying: None,
                question_expiry: None,
                question_price_thresholds: Vec::new(),
                question_period: None,
                question_named_outcomes: Vec::new(),
                question_settled_named_outcomes: Vec::new(),
                question_fallback_outcome: None,
                bucket_index: None,
                is_question_fallback: false,
                side_index: 0,
                side_name: "Yes".to_string(),
                outcome_name: "Recurring".to_string(),
                description: "Will BTC close green?".to_string(),
                class: None,
                underlying: None,
                expiry: None,
                target_price: None,
                period: None,
                quote_symbol: "USDH".to_string(),
                quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
                encoding: 950,
            }),
        }
    }

    fn context(day_vlm: f64) -> WatchlistContext {
        WatchlistContext {
            funding: None,
            prev_day_px: None,
            day_vlm: Some(day_vlm),
        }
    }

    fn payload(symbols: Vec<ExchangeSymbol>) -> ExchangeSymbolsPayload {
        ExchangeSymbolsPayload {
            symbols,
            loaded_from_cache: false,
            perp_meta_failed: false,
            spot_meta_failed: false,
            outcome_meta_failed: false,
        }
    }

    fn quick_order_form() -> QuickOrderForm {
        QuickOrderForm {
            price: 100.0,
            quantity: "2.5".to_string(),
            quantity_is_usd: false,
            percentage: 25.0,
            quantity_provenance: None,
            is_limit: true,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        }
    }

    #[test]
    fn failed_spot_metadata_retains_markets_but_disables_orders_until_verified() {
        let mut terminal = TradingTerminal::boot().0;
        let spot = spot_symbol("@107");
        let _task =
            terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("HYPE"), spot.clone()])));
        assert!(!terminal.spot_metadata_degraded);
        assert!(
            terminal
                .validate_exchange_symbol_orderable(&spot, "Active")
                .is_ok()
        );

        let _task = terminal.apply_symbols_loaded(Ok(ExchangeSymbolsPayload {
            symbols: vec![perp_symbol("HYPE")],
            loaded_from_cache: false,
            perp_meta_failed: false,
            spot_meta_failed: true,
            outcome_meta_failed: false,
        }));

        assert!(terminal.spot_metadata_degraded);
        let retained = terminal
            .exchange_symbol_for_key("@107")
            .expect("last-known spot symbol retained");
        let error = terminal
            .validate_exchange_symbol_orderable(retained, "Active")
            .expect_err("unverified spot metadata must fail closed");
        assert!(error.contains("temporarily unverified"));
        assert!(terminal.symbol_search_status.as_ref().is_some_and(
            |(message, is_error)| *is_error && message.contains("spot trading is disabled")
        ));

        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("HYPE"), spot])));

        assert!(!terminal.spot_metadata_degraded);
        let verified = terminal
            .exchange_symbol_for_key("@107")
            .expect("fresh spot symbol");
        assert!(
            terminal
                .validate_exchange_symbol_orderable(verified, "Active")
                .is_ok()
        );
        assert!(
            terminal.symbol_search_status.as_ref().is_some_and(
                |(message, is_error)| !*is_error && message.contains("available again")
            )
        );
    }

    #[test]
    fn cached_spot_metadata_is_visible_but_requires_immediate_live_verification() {
        let mut terminal = TradingTerminal::boot().0;
        let spot = spot_symbol("@107");
        let mut cached = payload(vec![perp_symbol("HYPE"), spot.clone()]);
        cached.loaded_from_cache = true;

        let task = terminal.apply_symbols_loaded(Ok(cached));

        assert!(terminal.exchange_symbol_for_key("@107").is_some());
        assert!(terminal.spot_metadata_degraded);
        assert!(terminal.exchange_symbols_refresh_inflight);
        assert!(task.units() >= 1, "live verification must be scheduled now");
        let cached_spot = terminal
            .exchange_symbol_for_key("@107")
            .expect("cached spot remains visible");
        let error = terminal
            .validate_exchange_symbol_orderable(cached_spot, "Active")
            .expect_err("cache provenance cannot authorize a spot order");
        assert!(error.contains("temporarily unverified"));
        assert!(
            terminal
                .symbol_search_status
                .as_ref()
                .is_some_and(|(message, is_error)| *is_error
                    && message.contains("live spot metadata is verified"))
        );

        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("HYPE"), spot])));

        assert!(!terminal.spot_metadata_degraded);
        assert!(!terminal.exchange_symbols_refresh_inflight);
        let verified_spot = terminal
            .exchange_symbol_for_key("@107")
            .expect("live-verified spot");
        assert!(
            terminal
                .validate_exchange_symbol_orderable(verified_spot, "Active")
                .is_ok()
        );
    }

    #[test]
    fn stale_symbols_result_does_not_mutate_metadata_or_release_current_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.symbols_loading = false;
        terminal.exchange_symbols = vec![perp_symbol("HYPE")];
        terminal.exchange_symbols_refresh_inflight = true;
        terminal.exchange_symbols_request_id = 8;
        terminal.spot_metadata_degraded = true;
        terminal.symbol_search_status =
            Some(("current metadata request pending".to_string(), true));

        let _task = terminal.update_symbol_search_market(Message::SymbolsLoaded(
            7,
            Ok(payload(vec![perp_symbol("ETH")])).into(),
        ));

        assert!(terminal.exchange_symbols_refresh_inflight);
        assert_eq!(terminal.exchange_symbols_request_id, 8);
        assert_eq!(terminal.exchange_symbols, vec![perp_symbol("HYPE")]);
        assert!(terminal.spot_metadata_degraded);
        assert_eq!(
            terminal.symbol_search_status,
            Some(("current metadata request pending".to_string(), true))
        );

        let _task = terminal.update_symbol_search_market(Message::SymbolsLoaded(
            8,
            Ok(payload(vec![perp_symbol("ETH")])).into(),
        ));

        assert!(!terminal.exchange_symbols_refresh_inflight);
        assert_ne!(terminal.exchange_symbols_request_id, 8);
        assert_eq!(terminal.exchange_symbols, vec![perp_symbol("ETH")]);
    }

    #[test]
    fn duplicate_cached_symbols_result_cannot_displace_live_verification_owner() {
        let mut terminal = TradingTerminal::boot().0;
        let spot = spot_symbol("@107");
        let mut cached = payload(vec![perp_symbol("HYPE"), spot]);
        cached.loaded_from_cache = true;
        terminal.symbols_loading = true;
        terminal.exchange_symbols_refresh_inflight = false;
        terminal.exchange_symbols_request_id = 10;

        let task = terminal
            .update_symbol_search_market(Message::SymbolsLoaded(10, Ok(cached.clone()).into()));

        let live_request_id = terminal.exchange_symbols_request_id;
        assert_ne!(live_request_id, 10);
        assert!(terminal.exchange_symbols_refresh_inflight);
        assert!(terminal.spot_metadata_degraded);
        assert!(task.units() >= 1, "live verification must be scheduled now");
        let status = terminal.symbol_search_status.clone();

        let _task =
            terminal.update_symbol_search_market(Message::SymbolsLoaded(10, Ok(cached).into()));

        assert_eq!(terminal.exchange_symbols_request_id, live_request_id);
        assert!(terminal.exchange_symbols_refresh_inflight);
        assert!(terminal.spot_metadata_degraded);
        assert_eq!(terminal.symbol_search_status, status);
    }

    #[test]
    fn exchange_symbols_request_generation_wraps_without_reusing_current_owner() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.symbols_loading = false;
        terminal.exchange_symbols_refresh_inflight = false;
        terminal.exchange_symbols_request_id = u64::MAX;

        let task = terminal.request_exchange_symbols_refresh();

        assert_eq!(terminal.exchange_symbols_request_id, 0);
        assert!(terminal.exchange_symbols_refresh_inflight);
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn cold_partial_load_preserves_saved_spot_selection() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "@107".to_string();
        terminal.active_symbol_display = "HYPE/USDC".to_string();

        let _task = terminal.apply_symbols_loaded(Ok(ExchangeSymbolsPayload {
            symbols: vec![perp_symbol("HYPE")],
            loaded_from_cache: false,
            perp_meta_failed: false,
            spot_meta_failed: true,
            outcome_meta_failed: false,
        }));

        assert_eq!(terminal.active_symbol, "@107");
        assert_eq!(terminal.active_symbol_display, "HYPE/USDC");
        assert!(terminal.spot_metadata_degraded);
    }

    #[test]
    fn perp_failure_does_not_discard_fresh_spot_metadata() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC")];

        let merged = terminal.merge_symbols_payload(ExchangeSymbolsPayload {
            symbols: vec![spot_symbol("@107")],
            loaded_from_cache: false,
            perp_meta_failed: true,
            spot_meta_failed: false,
            outcome_meta_failed: false,
        });

        assert!(
            merged
                .iter()
                .any(|symbol| symbol.market_type == MarketType::Perp && symbol.key == "BTC")
        );
        assert!(
            merged
                .iter()
                .any(|symbol| symbol.market_type == MarketType::Spot && symbol.key == "@107")
        );
    }

    #[test]
    fn successful_spot_metadata_migrates_legacy_widget_keys_atomically() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.order_books.clear();
        let mut book =
            OrderBookInstance::new(7, OrderBookSymbolMode::Fixed("@0".to_string()), 0.01);
        book.book_error = Some("legacy fetch failed".to_string());
        terminal.order_books.insert(7, book);

        terminal.spaghetti_charts.clear();
        let mut spaghetti = SpaghettiChartInstance::new_empty(8);
        spaghetti.canvas.series.push(Series {
            symbol: "@0".to_string(),
            display: "@0".to_string(),
            candles: vec![Candle::test_flat(1_000, 0.2)],
            color: iced::Color::BLACK,
            loaded: true,
        });
        terminal.spaghetti_charts.insert(8, spaghetti);

        terminal.live_watchlists.clear();
        terminal.live_watchlists.insert(
            9,
            LiveWatchlistInstance {
                id: 9,
                symbols: vec!["@0".to_string(), "PURR/USDC".to_string()],
                search_query: String::new(),
                sort_column: crate::config::LiveWatchlistSortColumn::default(),
                sort_direction: crate::config::SortDirection::default(),
                visible_columns: crate::config::default_live_watchlist_columns(),
                row_cache: Vec::new(),
            },
        );
        terminal.live_watchlist_contexts_loading = true;
        terminal.live_watchlist_contexts_request_id = 3;
        terminal.live_watchlist_contexts_request_symbols = vec!["@0".to_string()];
        terminal.live_watchlist_history_loading = true;
        terminal.live_watchlist_history_request_id = 4;
        terminal.live_watchlist_history_request_symbols = vec!["@0".to_string()];

        let task = terminal.apply_symbols_loaded(Ok(payload(vec![
            perp_symbol("HYPE"),
            canonical_purr_symbol(),
        ])));

        assert_eq!(
            terminal.order_books[&7].mode,
            OrderBookSymbolMode::Fixed("PURR/USDC".to_string())
        );
        assert!(terminal.order_books[&7].book_loading);
        assert!(terminal.order_books[&7].book_error.is_none());
        let series = &terminal.spaghetti_charts[&8].canvas.series;
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].symbol, "PURR/USDC");
        assert_eq!(series[0].display, "PURR/USDC");
        assert!(series[0].candles.is_empty());
        assert!(!series[0].loaded);
        assert_eq!(
            terminal.live_watchlists[&9].symbols,
            vec!["PURR/USDC".to_string()]
        );
        assert!(!terminal.live_watchlist_contexts_loading);
        assert!(terminal.live_watchlist_contexts_request_symbols.is_empty());
        assert_eq!(terminal.live_watchlist_contexts_request_id, 4);
        assert!(!terminal.live_watchlist_history_loading);
        assert!(terminal.live_watchlist_history_request_symbols.is_empty());
        assert_eq!(terminal.live_watchlist_history_request_id, 5);
        assert!(terminal.config_save_due_at.is_some());
        assert!(
            task.units() >= 2,
            "book and spaghetti data must be refetched"
        );
    }

    #[test]
    fn successful_spot_metadata_migrates_regular_chart_aliases_before_refetch() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();

        let context = crate::chart_state::ChartBackfillRequestContext::new(
            crate::config::ChartBackfillSource::Hyperliquid,
            0,
            0,
            terminal.chart_instance_generation,
        );
        let mut duplicate = ChartInstance::new(10, "@0".to_string(), Timeframe::H1);
        duplicate.set_secondary_symbol_identity("@0".to_string(), "@0".to_string());
        duplicate.candle_fetch_request = Some(TradingTerminal::build_candle_fetch_request(
            10,
            "@0",
            Timeframe::H1,
            context,
            None,
            0,
        ));
        duplicate.secondary_candle_fetch_request = Some(
            TradingTerminal::build_candle_fetch_request(10, "@0", Timeframe::H1, context, None, 0),
        );
        terminal.charts.insert(10, duplicate);

        let mut secondary = ChartInstance::new(11, "BTC".to_string(), Timeframe::H1);
        secondary.set_secondary_symbol_identity("@0".to_string(), "@0".to_string());
        secondary.secondary_candle_fetch_request = Some(
            TradingTerminal::build_candle_fetch_request(11, "@0", Timeframe::H1, context, None, 0),
        );
        terminal.charts.insert(11, secondary);

        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![
            perp_symbol("BTC"),
            canonical_purr_symbol(),
        ])));

        let duplicate = &terminal.charts[&10];
        assert_eq!(duplicate.symbol, "PURR/USDC");
        assert_eq!(duplicate.symbol_display, "PURR/USDC");
        assert_eq!(
            duplicate
                .candle_fetch_request
                .as_ref()
                .map(|request| request.symbol.as_str()),
            Some("PURR/USDC")
        );
        assert!(duplicate.secondary_symbol.is_none());
        assert!(duplicate.secondary_candle_fetch_request.is_none());

        let secondary = &terminal.charts[&11];
        assert_eq!(secondary.symbol, "BTC");
        assert_eq!(secondary.secondary_symbol.as_deref(), Some("PURR/USDC"));
        assert_eq!(
            secondary
                .secondary_candle_fetch_request
                .as_ref()
                .map(|request| request.symbol.as_str()),
            Some("PURR/USDC")
        );
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn symbol_search_context_filter_change_queues_current_scope_after_in_flight_result() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC"), perp_symbol("xyz:ETH")];
        terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
        terminal.symbol_search_market_filter = SymbolSearchMarketFilter::NativePerps;

        let _task = terminal.request_symbol_search_context_refresh(true);
        let stale_request_id = terminal.symbol_search_contexts_request_id;
        assert!(terminal.symbol_search_contexts_loading);
        assert_eq!(
            terminal.symbol_search_contexts_request_symbols,
            vec!["BTC".to_string()]
        );

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchMarketFilterChanged(
            SymbolSearchMarketFilter::Hip3,
        ));
        assert!(terminal.symbol_search_contexts_refresh_pending);

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            stale_request_id,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()).into(),
        ));

        assert!(
            terminal.symbol_search_contexts_loading,
            "queued refresh should start for the current HIP-3 scope"
        );
        assert!(!terminal.symbol_search_contexts_refresh_pending);
        assert_eq!(
            terminal.symbol_search_contexts_request_symbols,
            vec!["xyz:ETH".to_string()]
        );
        let current_request_id = terminal.symbol_search_contexts_request_id;

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            current_request_id,
            vec!["xyz:ETH".to_string()],
            11,
            Ok(HashMap::from([("xyz:ETH".to_string(), context(2.0))]).into()).into(),
        ));

        assert!(!terminal.symbol_search_contexts_loading);
        assert!(!terminal.symbol_search_ctxs.contains_key("BTC"));
        assert_eq!(
            terminal
                .symbol_search_ctxs
                .get("xyz:ETH")
                .map(|ctx| ctx.day_vlm),
            Some(Some(2.0))
        );
    }

    #[test]
    fn stale_symbol_search_context_result_is_ignored_after_current_completion() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC")];
        terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;

        let _task = terminal.request_symbol_search_context_refresh(true);
        let request_id = terminal.symbol_search_contexts_request_id;
        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            request_id,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()).into(),
        ));
        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            request_id,
            vec!["BTC".to_string()],
            11,
            Ok(HashMap::from([("BTC".to_string(), context(2.0))]).into()).into(),
        ));

        assert!(!terminal.symbol_search_contexts_loading);
        assert_eq!(
            terminal
                .symbol_search_ctxs
                .get("BTC")
                .map(|ctx| ctx.day_vlm),
            Some(Some(1.0))
        );
        assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, Some(10));
    }

    #[test]
    fn mismatched_symbol_search_context_scope_does_not_consume_current_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC"), perp_symbol("ETH")];
        terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
        terminal
            .symbol_search_ctxs
            .insert("BTC".to_string(), context(9.0));
        terminal.symbol_search_contexts_loading = true;
        terminal.symbol_search_contexts_request_id = 7;
        terminal.symbol_search_contexts_request_symbols = vec!["BTC".to_string()];
        terminal.symbol_search_contexts_refresh_pending = true;
        terminal.symbol_search_contexts_last_fetch_ms = Some(5);
        terminal.symbol_search_status = Some(("existing status".to_string(), true));

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            7,
            vec!["ETH".to_string()],
            10,
            Ok(HashMap::from([("ETH".to_string(), context(1.0))]).into()).into(),
        ));

        assert!(terminal.symbol_search_contexts_loading);
        assert_eq!(terminal.symbol_search_contexts_request_id, 7);
        assert_eq!(
            terminal.symbol_search_contexts_request_symbols,
            vec!["BTC".to_string()]
        );
        assert!(terminal.symbol_search_contexts_refresh_pending);
        assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, Some(5));
        assert_eq!(
            terminal
                .symbol_search_ctxs
                .get("BTC")
                .and_then(|context| context.day_vlm),
            Some(9.0)
        );
        assert!(!terminal.symbol_search_ctxs.contains_key("ETH"));
        assert_eq!(
            terminal.symbol_search_status,
            Some(("existing status".to_string(), true))
        );
    }

    #[test]
    fn symbol_search_context_result_keeps_only_requested_symbols() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC"), perp_symbol("ETH")];
        terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
        terminal.symbol_search_contexts_loading = true;
        terminal.symbol_search_contexts_request_id = 7;
        terminal.symbol_search_contexts_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            7,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::from([
                ("BTC".to_string(), context(1.0)),
                ("ETH".to_string(), context(2.0)),
            ])
            .into())
            .into(),
        ));

        assert_eq!(terminal.symbol_search_ctxs.len(), 1);
        assert!(terminal.symbol_search_ctxs.contains_key("BTC"));
        assert!(!terminal.symbol_search_ctxs.contains_key("ETH"));
    }

    #[test]
    fn symbol_search_context_success_clears_stale_omitted_requested_symbol() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC")];
        terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
        terminal
            .symbol_search_ctxs
            .insert("BTC".to_string(), context(9.0));
        terminal.symbol_search_contexts_loading = true;
        terminal.symbol_search_contexts_request_id = 7;
        terminal.symbol_search_contexts_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            7,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::new().into()).into(),
        ));

        assert!(!terminal.symbol_search_contexts_loading);
        assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, Some(10));
        assert!(!terminal.symbol_search_ctxs.contains_key("BTC"));
    }

    #[test]
    fn symbol_search_partial_context_keeps_omitted_requested_last_known_value() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC"), spot_symbol("@107")];
        terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
        terminal
            .symbol_search_ctxs
            .insert("@107".to_string(), context(9.0));
        terminal.symbol_search_contexts_loading = true;
        terminal.symbol_search_contexts_request_id = 7;
        terminal.symbol_search_contexts_request_symbols =
            vec!["BTC".to_string(), "@107".to_string()];

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            7,
            vec!["BTC".to_string(), "@107".to_string()],
            10,
            Ok(crate::api::WatchlistContextsResponse {
                contexts: HashMap::from([("BTC".to_string(), context(1.0))]),
                partial_errors: vec!["spot: HTTP 503".to_string()],
            })
            .into(),
        ));

        assert_eq!(
            terminal
                .symbol_search_ctxs
                .get("@107")
                .and_then(|ctx| ctx.day_vlm),
            Some(9.0)
        );
    }

    #[test]
    fn symbol_search_context_error_keeps_existing_cache_without_marking_fresh() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC")];
        terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
        terminal.symbol_search_contexts_last_fetch_ms = Some(10);
        terminal
            .symbol_search_ctxs
            .insert("BTC".to_string(), context(9.0));
        terminal.symbol_search_contexts_loading = true;
        terminal.symbol_search_contexts_request_id = 7;
        terminal.symbol_search_contexts_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            7,
            vec!["BTC".to_string()],
            20,
            Err("network".to_string()).into(),
        ));

        assert!(!terminal.symbol_search_contexts_loading);
        assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, Some(10));
        assert_eq!(
            terminal
                .symbol_search_ctxs
                .get("BTC")
                .map(|ctx| ctx.day_vlm),
            Some(Some(9.0))
        );
        assert_eq!(
            terminal
                .symbol_search_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("24h volume refresh failed: network", true))
        );
    }

    #[test]
    fn empty_symbol_search_scope_invalidates_in_flight_context_result() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("BTC")];
        terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;

        let _task = terminal.request_symbol_search_context_refresh(true);
        let stale_request_id = terminal.symbol_search_contexts_request_id;
        terminal.exchange_symbols.clear();
        let _task = terminal.request_symbol_search_context_refresh(true);

        assert!(!terminal.symbol_search_contexts_loading);
        assert!(terminal.symbol_search_contexts_request_symbols.is_empty());
        assert!(terminal.symbol_search_ctxs.is_empty());
        assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, None);

        let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
            stale_request_id,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()).into(),
        ));

        assert!(terminal.symbol_search_ctxs.is_empty());
        assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, None);
    }

    #[test]
    fn symbols_loaded_refreshes_existing_outcome_chart_display_without_key_change() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "#950".to_string();
        terminal.active_symbol_display = "#950".to_string();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "#950".to_string(), Timeframe::H1));

        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));

        let expected_display = "YES: Will BTC close green?";
        assert_eq!(terminal.active_symbol, "#950");
        assert_eq!(terminal.active_symbol_display, expected_display);
        let chart = terminal.charts.get(&7).expect("chart");
        assert_eq!(chart.symbol, "#950");
        assert_eq!(chart.symbol_display, expected_display);
    }

    #[test]
    fn symbols_loaded_clears_stale_macro_candles_when_chart_key_is_canonicalized() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();

        let mut canonical = perp_symbol("xyz:BTC");
        canonical.ticker = "BTC".to_string();

        let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        chart.chart.hourly_candles = vec![Candle::test_flat(500, 50.0)];
        chart.chart.daily_candles = vec![Candle::test_flat(1_000, 100.0)];
        chart.chart.weekly_candles = vec![Candle::test_flat(2_000, 200.0)];
        chart.chart.monthly_candles = vec![Candle::test_flat(3_000, 300.0)];
        terminal.charts.insert(7, chart);

        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![canonical])));

        let chart = terminal.charts.get(&7).expect("chart");
        assert_eq!(chart.symbol, "xyz:BTC");
        assert!(chart.chart.hourly_candles.is_empty());
        assert!(chart.chart.daily_candles.is_empty());
        assert!(chart.chart.weekly_candles.is_empty());
        assert!(chart.chart.monthly_candles.is_empty());
        assert_eq!(chart.macro_candles_request_id, 1);
    }

    #[test]
    fn symbols_loaded_disarms_hud_when_chart_symbol_key_rewrites() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();

        let mut canonical = perp_symbol("xyz:BTC");
        canonical.ticker = "BTC".to_string();

        let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        chart
            .chart
            .set_crosshair_style(crate::config::ChartCrosshairStyle::Hud);
        chart.chart.set_hud_armed_at(true, 1_000);
        assert!(chart.chart.hud_armed());
        terminal.charts.insert(7, chart);

        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![canonical])));

        let chart = terminal.charts.get(&7).expect("chart");
        assert_eq!(chart.symbol, "xyz:BTC");
        assert_eq!(chart.chart.symbol_key, "xyz:BTC");
        assert!(!chart.chart.hud_armed());
    }

    #[test]
    fn symbols_loaded_resets_quick_order_when_chart_symbol_key_rewrites() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();

        let mut canonical = perp_symbol("xyz:BTC");
        canonical.ticker = "BTC".to_string();

        let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        chart.set_quick_order(quick_order_form());
        chart.track_last_price_update(Some(100.0), 101.0, 1_000);
        chart.heatmap_last_fetch = Some(HeatmapFetchParams {
            coin: "BTC".to_string(),
            min_price: 90.0,
            max_price: 110.0,
            start_time: 1,
            end_time: 2,
        });
        chart.heatmap_status = Some(("stale heatmap".to_string(), true));
        chart.heatmap_fetching = true;
        chart.heatmap_data = Some(LiquidationHeatmap {
            rects: Vec::new(),
            max_abs_usd: 1.0,
        });
        chart.liquidation_status = Some(("stale liquidations".to_string(), true));
        chart.liquidation_fetching = true;
        chart.liquidation_pending_key = Some("BTC".to_string());
        chart.liquidation_data = Some(LiquidationLevel {
            coin: "BTC".to_string(),
            min: 90.0,
            max: 110.0,
            liquidations: Vec::new(),
        });
        chart.chart.liquidation_buckets = vec![LiquidationBucket {
            price_center: 100.0,
            long_coins: 1.0,
            short_coins: 0.0,
            long_usd: 100.0,
            short_usd: 0.0,
        }];
        chart.chart.funding_rates = vec![FundingRatePoint {
            time_ms: 1,
            rate: 0.01,
        }];
        chart.chart.funding_status = Some(("stale funding".to_string(), false));
        terminal.charts.insert(7, chart);
        terminal
            .chart_quick_order_surface
            .insert(7, ChartSurfaceId::Docked(7));

        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![canonical])));

        let chart = terminal.charts.get(&7).expect("chart");
        assert_eq!(chart.symbol, "xyz:BTC");
        assert_eq!(chart.chart.symbol_key, "xyz:BTC");
        assert!(chart.quick_order.is_none());
        assert!(!chart.chart.quick_order_open);
        assert_eq!(chart.last_quick_order_symbol, "");
        assert!(chart.last_price_flash.is_none());
        assert!(chart.heatmap_last_fetch.is_none());
        assert!(chart.heatmap_status.is_none());
        assert!(!chart.heatmap_fetching);
        assert!(chart.heatmap_data.is_none());
        assert!(chart.liquidation_status.is_none());
        assert!(!chart.liquidation_fetching);
        assert!(chart.liquidation_pending_key.is_none());
        assert!(chart.liquidation_data.is_none());
        assert!(chart.chart.liquidation_buckets.is_empty());
        assert!(chart.chart.funding_rates.is_empty());
        assert!(chart.chart.funding_status.is_none());
        assert!(!terminal.chart_quick_order_surface.contains_key(&7));
    }

    #[test]
    fn symbols_loaded_keeps_outcome_labels_when_outcome_meta_fails_but_rejects_orderability() {
        let mut terminal = TradingTerminal::boot().0;
        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));
        assert_eq!(terminal.exchange_symbols.len(), 1);

        let _task = terminal.apply_symbols_loaded(Ok(ExchangeSymbolsPayload {
            symbols: Vec::new(),
            loaded_from_cache: false,
            perp_meta_failed: false,
            spot_meta_failed: false,
            outcome_meta_failed: true,
        }));

        assert_eq!(
            terminal.exchange_symbols.len(),
            1,
            "previously loaded outcome symbols must survive a failed outcomeMeta refresh"
        );
        assert_eq!(terminal.exchange_symbols[0].key, "#950");
        assert_eq!(
            terminal.exchange_symbols[0].display_name.as_deref(),
            Some("YES: Will BTC close green?")
        );
        assert!(terminal.exchange_symbols[0].outcome.is_none());
        assert!(!terminal.exchange_symbols[0].is_user_selectable_market());
        assert!(!terminal.exchange_symbol_is_orderable(&terminal.exchange_symbols[0]));
        assert_eq!(
            terminal.display_name_for_symbol("#950"),
            "YES: Will BTC close green?"
        );
        assert_eq!(
            terminal
                .symbol_search_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some((
                "Outcome market metadata failed to load; retrying shortly",
                true
            ))
        );
    }

    #[test]
    fn symbols_loaded_records_outcome_display_labels() {
        let mut terminal = TradingTerminal::boot().0;
        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));

        assert_eq!(
            terminal
                .outcome_display_labels
                .get("#950")
                .map(String::as_str),
            Some("YES: Will BTC close green?")
        );

        // The cached label keeps resolving the coin after the market expires
        // and disappears from outcomeMeta.
        let _task = terminal.apply_symbols_loaded(Ok(payload(Vec::new())));
        assert_eq!(
            terminal.display_name_for_symbol("#950"),
            "YES: Will BTC close green?"
        );
    }

    #[test]
    fn symbols_loaded_refreshes_spaghetti_series_displays() {
        let mut terminal = TradingTerminal::boot().0;
        let mut inst = crate::spaghetti_state::SpaghettiChartInstance::new_empty(3);
        inst.canvas.series.push(crate::spaghetti::Series {
            symbol: "#950".to_string(),
            display: "#950".to_string(),
            candles: Vec::new(),
            color: iced::Color::WHITE,
            loaded: false,
        });
        terminal.spaghetti_charts.insert(3, inst);

        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));

        let inst = terminal.spaghetti_charts.get(&3).expect("spaghetti chart");
        assert_eq!(inst.canvas.series[0].display, "YES: Will BTC close green?");
    }

    #[test]
    fn symbols_load_error_after_successful_load_keeps_symbols_and_stays_quiet() {
        let mut terminal = TradingTerminal::boot().0;
        let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));
        terminal.symbol_search_status = None;

        let _task = terminal.apply_symbols_loaded(Err("network down".to_string()));

        assert_eq!(terminal.exchange_symbols.len(), 1);
        assert!(
            terminal.symbol_search_status.is_none(),
            "background refresh failures must not surface error status"
        );
        assert!(!terminal.symbols_loading);
    }

    #[test]
    fn symbols_load_error_before_initial_load_redacts_status_and_toast() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols.clear();
        terminal.toasts.clear();

        let _task = terminal.apply_symbols_loaded(Err(
            "symbol fetch failed: api_key=key-secret auth_token=token-secret".to_string(),
        ));

        let status = terminal.symbol_search_status.as_ref().expect("status");
        assert!(status.1);
        assert!(status.0.contains("api_key=<redacted>"));
        assert!(status.0.contains("auth_token=<redacted>"));
        assert!(!status.0.contains("key-secret"));
        assert!(!status.0.contains("token-secret"));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert_eq!(toast.message, status.0);
    }
}
