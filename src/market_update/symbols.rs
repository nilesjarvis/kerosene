mod contexts;
mod controls;
mod outcome_volumes;
mod resolution;

use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::config::MarketUniverseConfig;
use crate::market_state::SymbolSearchMarketFilter;
use crate::message::Message;

use self::contexts::apply_contexts_loaded;
use self::controls::{apply_hip3_dex_filter, apply_market_filter, toggle_favourite_symbol};
use resolution::resolve_exchange_symbol;

use iced::Task;

impl TradingTerminal {
    pub(super) fn update_symbol_search_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleFavourite(key) => self.toggle_market_favourite(key),
            Message::SymbolsLoaded(result) => self.apply_symbols_loaded(result),
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
            Message::SymbolSearchContextsLoaded(requested_at, result) => {
                apply_contexts_loaded(
                    &mut self.symbol_search_contexts_loading,
                    &mut self.symbol_search_contexts_last_fetch_ms,
                    &mut self.symbol_search_ctxs,
                    &mut self.symbol_search_status,
                    requested_at,
                    result,
                );
                self.refresh_symbol_search_results();
                Task::none()
            }
            Message::OutcomeSearchChanged(query) => {
                self.outcome_search_query = query;
                Task::none()
            }
            Message::OutcomeVolumesLoaded(result) => self.apply_outcome_volumes_loaded(result),
            Message::SymbolSelected(key) => self.select_market_symbol(key),
            _ => Task::none(),
        }
    }

    fn toggle_market_favourite(&mut self, key: String) -> Task<Message> {
        if self.symbol_key_is_hidden(&key) {
            self.symbol_search_status = Some((format!("{key} is hidden by Settings > Risk"), true));
            return Task::none();
        }
        toggle_favourite_symbol(&mut self.favourite_symbols, key);
        self.refresh_symbol_search_results();
        self.persist_config();
        self.request_ticker_tape_context_refresh(true)
    }

    fn apply_symbols_loaded(
        &mut self,
        result: Result<Vec<crate::api::ExchangeSymbol>, String>,
    ) -> Task<Message> {
        match result {
            Ok(symbols) => {
                self.exchange_symbols = symbols;
                let mut market_universe_changed = false;
                let normalized_universe =
                    self.normalize_market_universe_selection(self.market_universe.clone());
                if normalized_universe != self.market_universe {
                    self.market_universe = normalized_universe;
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

                let mut tasks = Vec::new();
                tasks.extend(self.mids_bootstrap_tasks());

                let active_symbol = self.active_symbol.clone();
                match self.restored_active_symbol_key(&active_symbol) {
                    Some(valid_key) if valid_key != self.active_symbol => {
                        tasks.push(self.switch_active_symbol_internal(valid_key));
                    }
                    Some(_) => {}
                    None => {
                        self.apply_active_symbol_selection(String::new(), String::new());
                        self.order_status =
                            Some(("No tradable market symbols are available".into(), true));
                    }
                }

                for (id, inst) in self.charts.iter_mut() {
                    let key = inst.symbol.clone();
                    let symbol = resolve_exchange_symbol(&self.exchange_symbols, &key);

                    if let Some(valid) = symbol
                        && valid.key != inst.symbol
                    {
                        inst.symbol = valid.key.clone();
                        inst.symbol_display = valid
                            .display_name
                            .clone()
                            .unwrap_or_else(|| valid.ticker.clone());
                        inst.chart.status = ChartStatus::Loading;
                        inst.chart.candles.clear();
                        inst.chart.candle_cache.clear();
                        inst.asset_ctx = None;
                        inst.candle_fetch_error = None;
                        inst.last_price_flash = None;
                        let request = Self::build_candle_fetch_request(
                            *id,
                            &valid.key,
                            inst.interval,
                            None,
                            0,
                        );
                        inst.candle_fetch_request = Some(request.clone());
                        let mut chart_tasks = vec![Self::fetch_candles_task(request)];
                        chart_tasks.extend(Self::fetch_macro_candles_tasks(*id, &valid.key));
                        tasks.push(Task::batch(chart_tasks));
                    }
                }

                tasks.push(self.scrub_hidden_symbol_state());
                self.refresh_symbol_search_results();
                self.refresh_live_watchlist_row_caches();
                tasks.push(self.request_symbol_search_context_refresh(false));
                tasks.push(self.request_ticker_tape_context_refresh(true));
                tasks.push(self.request_outcome_volume_refresh());
                if market_universe_changed {
                    tasks.push(self.refresh_account_data());
                }

                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }
            Err(error) => {
                self.symbols_loading = false;
                let message = format!("Symbol load failed: {error}");
                self.symbol_search_status = Some((message.clone(), true));
                self.push_toast(message, true);
            }
        }

        Task::none()
    }

    fn select_market_symbol(&mut self, key: String) -> Task<Message> {
        if self.active_symbol == key {
            return Task::none();
        }

        self.switch_active_symbol_internal(key)
    }
}
