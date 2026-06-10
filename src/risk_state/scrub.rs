use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartInstance;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use iced::Task;

// ---------------------------------------------------------------------------
// Muted Ticker State Scrubbing
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn clear_chart_for_muted_symbol(instance: &mut ChartInstance) {
        instance.symbol.clear();
        instance.symbol_display.clear();
        instance.chart.status = ChartStatus::Loading;
        instance.chart.candles.clear();
        instance.chart.candle_cache.clear();
        instance.chart.active_position = None;
        instance.chart.active_orders.clear();
        instance.chart.trade_markers.clear();
        instance.chart.clear_hud_armed();
        instance.chart.set_market_reference_price(None);
        instance.asset_ctx = None;
        instance.candle_fetch_request = None;
        instance.candle_fetch_error = None;
        instance.editor_open = false;
        instance.editor_search_query.clear();
        instance.editor_selected_index = None;
        instance.heatmap_last_fetch = None;
        instance.heatmap_viewport = None;
        instance.heatmap_status = None;
        instance.heatmap_fetching = false;
        Self::clear_heatmap_display(instance);
        Self::clear_liquidation_display(instance);
        Self::clear_earnings_display(instance);
    }

    pub(crate) fn scrub_muted_ticker_state(&mut self) -> Task<Message> {
        self.scrub_hidden_symbol_state()
    }

    pub(crate) fn scrub_hidden_symbol_state(&mut self) -> Task<Message> {
        let exchange_symbols = self.exchange_symbols.clone();
        let muted_tickers = self.muted_tickers.clone();
        let market_universe = self.market_universe.clone();
        let denomination_rate_key = self.display_denomination_rate_symbol_key();
        let is_hidden = |symbol: &str| {
            Self::symbol_key_is_hidden_with(
                &exchange_symbols,
                &muted_tickers,
                &market_universe,
                symbol,
            )
        };
        let is_hidden_cache = |symbol: &str| {
            denomination_rate_key.as_deref() != Some(symbol)
                && Self::symbol_key_is_hidden_with(
                    &exchange_symbols,
                    &muted_tickers,
                    &market_universe,
                    symbol,
                )
        };

        self.favourite_symbols.retain(|symbol| !is_hidden(symbol));
        for watchlist in self.live_watchlists.values_mut() {
            watchlist.symbols.retain(|symbol| !is_hidden(symbol));
        }
        self.symbol_search_ctxs
            .retain(|symbol, _| !is_hidden(symbol));
        self.live_watchlist_ctxs
            .retain(|symbol, _| !is_hidden(symbol));
        self.live_watchlist_history
            .retain(|symbol, _| !is_hidden(symbol));
        self.live_watchlist_history_loaded_at
            .retain(|symbol, _| !is_hidden(symbol));
        self.all_mids.retain(|symbol, _| !is_hidden_cache(symbol));
        self.all_mids_updated_at_ms
            .retain(|symbol, _| !is_hidden_cache(symbol));
        self.live_watchlist_flashes
            .retain(|symbol, _| !is_hidden_cache(symbol));

        if self.account_data.is_some() {
            self.sync_all_chart_overlays();
        }
        for state in self.wallet_detail_windows.values_mut() {
            if let Some(data) = state.data.take() {
                state.data = Some(Self::filter_wallet_details_for_hidden_symbols_with(
                    &exchange_symbols,
                    &muted_tickers,
                    &market_universe,
                    data,
                ));
            }
        }

        self.tracked_trades.retain(|trade| !is_hidden(&trade.coin));
        self.tracked_trade_seen_keys.clear();
        self.tracked_trade_seen_order.clear();
        let remaining_trades: Vec<_> = self.tracked_trades.iter().cloned().collect();
        for trade in &remaining_trades {
            let _ = self.remember_tracked_trade_event(trade);
        }

        self.liquidations.retain(|liq| !is_hidden(&liq.coin));
        self.recompute_liquidation_buckets();

        if self.close_menu_coin.as_deref().is_some_and(&is_hidden) {
            self.close_menu_coin = None;
        }

        // Rebound books track the active symbol from here on; re-seed the
        // per-symbol bookkeeping so the muted symbol's tick, options basis,
        // and failure state do not leak into the new binding.
        let active_default_tick = self
            .resolve_mid_for_symbol(&self.active_symbol)
            .map(crate::helpers::default_tick_for_price)
            .unwrap_or(0.01);
        for order_book in self.order_books.values_mut() {
            let symbol = match &order_book.mode {
                OrderBookSymbolMode::Active => self.active_symbol.clone(),
                OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
            };
            if is_hidden(&symbol) {
                let was_fixed = matches!(order_book.mode, OrderBookSymbolMode::Fixed(_));
                order_book.mode = OrderBookSymbolMode::Active;
                order_book.set_book(OrderBook::empty());
                order_book.asset_ctx = None;
                order_book.spread_history.clear();
                order_book.clear_mid_price_history();
                order_book.clear_book_request();
                order_book.book_loading = false;
                order_book.book_error = None;
                order_book.book_failure_toasted = false;
                if was_fixed {
                    order_book.reset_tick_options_basis();
                    order_book.set_tick_size(active_default_tick);
                }
            }
        }

        for instance in self.charts.values_mut() {
            if !instance.symbol.is_empty() && is_hidden(&instance.symbol) {
                Self::clear_chart_for_muted_symbol(instance);
            }
        }

        for inst in self.spaghetti_charts.values_mut() {
            inst.canvas
                .series
                .retain(|series| !is_hidden(&series.symbol));
            inst.editor_search_query.clear();
        }

        let mut session_data_refresh_ids = Vec::new();
        let fallback_session_symbol = self.visible_session_data_symbol("");
        for inst in self.session_data.values_mut() {
            if !inst.symbol.is_empty() && is_hidden(&inst.symbol) {
                inst.symbol = fallback_session_symbol.clone();
                inst.search_query.clear();
                inst.symbol_picker_open = false;
                inst.clear_history();
                session_data_refresh_ids.push(inst.id);
            }
        }

        self.refresh_live_watchlist_row_caches();

        let mut tasks = session_data_refresh_ids
            .into_iter()
            .map(|id| self.request_session_data_refresh(id, true))
            .collect::<Vec<_>>();

        if !self.active_symbol.is_empty() && is_hidden(&self.active_symbol) {
            if let Some(fallback) = self.fallback_unmuted_symbol_key() {
                tasks.push(self.switch_active_symbol_internal(fallback));
            } else {
                self.apply_active_symbol_selection(String::new(), String::new());
                self.order_status = Some(("No visible market symbols are available".into(), true));
            }
        }

        Task::batch(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
        Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::session_data_state::{SessionDataInstance, SessionDataLookback};

    fn perp_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 50,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn account_data_with_position(coin: &str) -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: vec![AssetPosition {
                    position: Position {
                        coin: coin.to_string(),
                        szi: "1".to_string(),
                        entry_px: "100".to_string(),
                        position_value: "100".to_string(),
                        unrealized_pnl: "0".to_string(),
                        liquidation_px: None,
                        leverage: PositionLeverage {
                            leverage_type: "cross".to_string(),
                            value: 1,
                        },
                        margin_used: "0".to_string(),
                        cum_funding: None,
                    },
                    liquidation_px: None,
                }],
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: 1,
        }
    }

    #[test]
    fn scrub_hidden_symbol_state_refreshes_session_data_when_active_symbol_changes() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "HYPE".to_string();
        terminal.exchange_symbols = vec![perp_symbol("HYPE"), perp_symbol("BTC")];
        terminal.muted_tickers.insert("HYPE".to_string());
        terminal.session_data.insert(
            4,
            SessionDataInstance::new(4, "HYPE".to_string(), SessionDataLookback::FourWeeks),
        );

        let _task = terminal.scrub_hidden_symbol_state();

        let instance = terminal.session_data.get(&4).expect("session data");
        assert_eq!(instance.symbol, "BTC");
        assert!(instance.loading);
        assert!(instance.pending_request.is_some());
        assert_eq!(terminal.active_symbol, "BTC");
    }

    #[test]
    fn scrub_hidden_symbol_state_keeps_hidden_account_positions() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![perp_symbol("HYPE"), perp_symbol("BTC")];
        terminal.account_data = Some(account_data_with_position("HYPE"));
        terminal.muted_tickers.insert("HYPE".to_string());

        let _task = terminal.scrub_hidden_symbol_state();

        let positions = &terminal
            .account_data
            .as_ref()
            .expect("account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "HYPE");
    }
}
