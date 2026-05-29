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

        if let Some(data) = self.account_data.take() {
            self.account_data = Some(Self::filter_account_data_for_hidden_symbols_with(
                &exchange_symbols,
                &muted_tickers,
                &market_universe,
                data,
            ));
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

        for order_book in self.order_books.values_mut() {
            let symbol = match &order_book.mode {
                OrderBookSymbolMode::Active => self.active_symbol.clone(),
                OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
            };
            if is_hidden(&symbol) {
                order_book.mode = OrderBookSymbolMode::Active;
                order_book.set_book(OrderBook::empty());
                order_book.asset_ctx = None;
                order_book.spread_history.clear();
                order_book.clear_mid_price_history();
                order_book.book_loading = false;
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

        self.refresh_live_watchlist_row_caches();

        if !self.active_symbol.is_empty() && is_hidden(&self.active_symbol) {
            if let Some(fallback) = self.fallback_unmuted_symbol_key() {
                return self.switch_active_symbol_internal(fallback);
            }
            self.apply_active_symbol_selection(String::new(), String::new());
            self.order_status = Some(("No visible market symbols are available".into(), true));
        }

        Task::none()
    }
}
