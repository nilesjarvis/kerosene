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
            Message::OutcomeMarketGroupToggled(key) => {
                if !self.outcome_collapsed_market_groups.insert(key.clone()) {
                    self.outcome_collapsed_market_groups.remove(&key);
                }
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
                self.refresh_telegram_ticker_mentions();
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
                    Some(valid_key) => {
                        if let Some(symbol) =
                            resolve_exchange_symbol(&self.exchange_symbols, &valid_key)
                        {
                            self.active_symbol_display = Self::exchange_symbol_display_name(symbol);
                        }
                    }
                    None => {
                        self.apply_active_symbol_selection(String::new(), String::new());
                        self.order_status =
                            Some(("No tradable market symbols are available".into(), true));
                    }
                }

                let chart_backfill_source = self.chart_backfill_source;
                let hydromancer_api_key = self.hydromancer_api_key.trim().to_string();
                for (id, inst) in self.charts.iter_mut() {
                    let key = inst.symbol.clone();
                    let symbol = resolve_exchange_symbol(&self.exchange_symbols, &key);

                    if let Some(valid) = symbol {
                        let display = Self::exchange_symbol_display_name(valid);
                        if inst.symbol_display != display {
                            inst.symbol_display = display.clone();
                            inst.chart.set_symbol_label(display);
                        }

                        if valid.key != inst.symbol {
                            inst.symbol = valid.key.clone();
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
                                chart_backfill_source,
                                None,
                                0,
                            );
                            inst.candle_fetch_request = Some(request.clone());
                            let mut chart_tasks = vec![Self::fetch_candles_task(
                                request,
                                hydromancer_api_key.clone(),
                            )];
                            chart_tasks.extend(Self::fetch_macro_candles_tasks(*id, &valid.key));
                            tasks.push(Task::batch(chart_tasks));
                        }
                    }
                }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

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

    #[test]
    fn symbols_loaded_refreshes_existing_outcome_chart_display_without_key_change() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "#950".to_string();
        terminal.active_symbol_display = "#950".to_string();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "#950".to_string(), Timeframe::H1));

        let _task = terminal.apply_symbols_loaded(Ok(vec![outcome_symbol("#950")]));

        let expected_display = "YES: Will BTC close green?";
        assert_eq!(terminal.active_symbol, "#950");
        assert_eq!(terminal.active_symbol_display, expected_display);
        let chart = terminal.charts.get(&7).expect("chart");
        assert_eq!(chart.symbol, "#950");
        assert_eq!(chart.symbol_display, expected_display);
    }
}
