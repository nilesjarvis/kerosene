use super::resolution::resolve_exchange_symbol;
use crate::api::{ExchangeSymbolsPayload, MarketType};
use crate::app_state::TradingTerminal;
use crate::config::MarketUniverseConfig;
use crate::helpers::redact_sensitive_response_text;
use crate::market_state::SymbolSearchMarketFilter;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn request_exchange_symbols_refresh(&mut self) -> Task<Message> {
        if self.symbols_loading || self.exchange_symbols_refresh_inflight {
            return Task::none();
        }
        self.exchange_symbols_refresh_inflight = true;
        Task::perform(crate::api::fetch_exchange_symbols(), Message::SymbolsLoaded)
    }

    /// A failed metadata request leaves that market type absent from the
    /// payload. Retained spot symbols stay visible but are fail-closed for new
    /// orders; retained outcome terms remain visible for inspection and
    /// cancellation until fresh metadata verifies them again.
    pub(super) fn merge_symbols_payload(
        &self,
        payload: ExchangeSymbolsPayload,
    ) -> Vec<crate::api::ExchangeSymbol> {
        let ExchangeSymbolsPayload {
            mut symbols,
            loaded_from_cache,
            perp_meta_failed,
            spot_meta_failed,
            outcome_meta_failed,
            ..
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
                        if let Some(info) = &mut symbol.outcome {
                            info.contract.verified = false;
                        }
                        symbol
                    }),
            );
        }
        if perp_meta_failed || spot_meta_failed || outcome_meta_failed {
            symbols.sort_by(|a, b| a.ticker.cmp(&b.ticker));
        }
        if loaded_from_cache {
            for symbol in &mut symbols {
                if let Some(info) = &mut symbol.outcome {
                    info.contract.verified = false;
                }
            }
        }
        symbols
    }

    /// Remember the display label of every loaded outcome market so fills,
    /// journal entries, and balances keep their names after the market
    /// expires and disappears from outcomeMeta.
    pub(super) fn record_outcome_display_labels(&mut self) {
        let mut changed = false;
        for symbol in self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Outcome)
        {
            let label = Self::exchange_symbol_display_name(symbol);
            if self.outcome_display_labels.get(&symbol.key) != Some(&label) {
                self.outcome_display_labels
                    .insert(symbol.key.clone(), label);
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

    pub(super) fn apply_symbols_loaded(
        &mut self,
        result: Result<ExchangeSymbolsPayload, String>,
    ) -> Task<Message> {
        self.exchange_symbols_refresh_inflight = false;
        match result {
            Ok(mut payload) => {
                let loaded_from_cache = payload.loaded_from_cache;
                let perp_meta_failed = payload.perp_meta_failed;
                let spot_meta_failed = payload.spot_meta_failed;
                let outcome_meta_failed = payload.outcome_meta_failed;
                let mut perp_dexes_changed = false;
                if !perp_meta_failed && let Some(dexes) = payload.perp_dexes.take() {
                    perp_dexes_changed = self.perp_dexes != dexes;
                    self.perp_dexes = dexes;
                }
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
                        "Cached markets are visible while live metadata is verified; spot and outcome trading remain disabled until verification succeeds"
                            .to_string(),
                        true,
                    ));
                    self.exchange_symbols_refresh_inflight = true;
                    initial_tasks.push(Task::perform(
                        crate::api::fetch_exchange_symbols(),
                        Message::SymbolsLoaded,
                    ));
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
                } else if outcome_meta_failed {
                    self.symbol_search_status = Some((
                        "Outcome market metadata failed to load; retrying shortly".to_string(),
                        true,
                    ));
                }
                // Cached/partial metadata cannot prove that a saved DEX disappeared.
                let normalized_universe = if loaded_from_cache || perp_meta_failed {
                    self.market_universe.clone()
                } else {
                    self.normalize_market_universe_selection(self.market_universe.clone())
                };
                let market_universe_changed = normalized_universe != self.market_universe;
                if !symbols_changed
                    && !perp_dexes_changed
                    && !market_universe_changed
                    && !self.exchange_symbols.is_empty()
                {
                    self.symbols_loading = false;
                    return Task::batch(initial_tasks);
                }
                self.record_outcome_display_labels();
                self.telegram_feed
                    .rebuild_ticker_mention_resolver(&self.exchange_symbols);
                self.refresh_telegram_ticker_mentions();
                if market_universe_changed {
                    self.market_universe = normalized_universe;
                    self.clear_percentage_order_quantity();
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

                tasks.extend(self.reconcile_chart_symbol_metadata());

                self.refresh_spaghetti_series_displays();
                tasks.push(self.reconcile_session_data_symbols());
                tasks.push(self.refresh_enabled_earnings_charts());
                self.refresh_symbol_search_results();
                self.refresh_live_watchlist_row_caches();
                tasks.push(self.request_symbol_search_context_refresh(false));
                tasks.push(self.request_ticker_tape_context_refresh(true));
                tasks.push(self.request_outcome_volume_refresh());
                tasks.push(self.request_screener_data_refresh(true));
                if market_universe_changed {
                    // Metadata refreshes also discover listings and update
                    // labels. Reload all widgets only when the selected market
                    // universe changes; symbol migrations are handled above.
                    tasks.push(self.reconcile_market_universe_state());
                }
                if market_universe_changed || perp_dexes_changed {
                    tasks.push(self.refresh_account_data());
                }

                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }
            Err(error) => {
                self.symbols_loading = false;
                for symbol in &mut self.exchange_symbols {
                    if let Some(info) = &mut symbol.outcome {
                        info.contract.verified = false;
                    }
                }
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
}
