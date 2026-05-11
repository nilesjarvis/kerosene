mod contexts;
mod controls;
mod resolution;

use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
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
            Message::SymbolSelected(key) => self.select_market_symbol(key),
            _ => Task::none(),
        }
    }

    fn toggle_market_favourite(&mut self, key: String) -> Task<Message> {
        if self.is_ticker_muted(&key) {
            self.symbol_search_status = Some((format!("{key} is muted in Settings > Risk"), true));
            return Task::none();
        }
        toggle_favourite_symbol(&mut self.favourite_symbols, key);
        self.refresh_symbol_search_results();
        self.persist_config();
        Task::none()
    }

    fn apply_symbols_loaded(
        &mut self,
        result: Result<Vec<crate::api::ExchangeSymbol>, String>,
    ) -> Task<Message> {
        match result {
            Ok(symbols) => {
                self.exchange_symbols = symbols;
                self.refresh_symbol_search_results();
                self.symbols_loading = false;

                let key = self.active_symbol.clone();
                let resolved_key = resolve_exchange_symbol(&self.exchange_symbols, &key)
                    .map(|symbol| symbol.key.clone());

                let mut tasks = Vec::new();
                tasks.extend(self.mids_bootstrap_tasks());

                if let Some(valid_key) = resolved_key
                    && valid_key != self.active_symbol
                {
                    tasks.push(self.switch_active_symbol_internal(valid_key));
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

                tasks.push(self.scrub_muted_ticker_state());
                self.refresh_symbol_search_results();
                self.refresh_live_watchlist_row_caches();
                tasks.push(self.request_symbol_search_context_refresh(false));

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
