use crate::api::{MarketType, OrderBook};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn switch_active_symbol_internal(&mut self, key: String) -> Task<Message> {
        let sym = self
            .exchange_symbols
            .iter()
            .find(|s| s.key == key)
            .or_else(|| {
                self.exchange_symbols
                    .iter()
                    .find(|s| s.ticker == key && s.market_type == MarketType::Perp)
            })
            .or_else(|| self.exchange_symbols.iter().find(|s| s.ticker == key));

        let valid_key = sym.map(|s| s.key.clone()).unwrap_or(key.clone());
        if self.is_ticker_muted(&valid_key) {
            self.order_status = Some((format!("{valid_key} is muted in Settings > Risk"), true));
            self.symbol_search_status =
                Some((format!("{valid_key} is muted in Settings > Risk"), true));
            return Task::none();
        }
        let valid_is_outcome =
            sym.is_some_and(|s| s.market_type == MarketType::Outcome) || valid_key.starts_with('#');
        let display = sym
            .map(Self::exchange_symbol_display_name)
            .unwrap_or_else(|| {
                valid_key
                    .split(':')
                    .nth(1)
                    .unwrap_or(&valid_key)
                    .to_string()
            });

        self.active_symbol = valid_key.clone();
        self.active_symbol_display = display.clone();
        self.refresh_order_price_for_symbol(&valid_key);
        if valid_is_outcome {
            self.order_quantity_is_usd = false;
            self.order_quantity = Self::sanitize_outcome_quantity_input(&self.order_quantity);
        }
        for inst in self.order_books.values_mut() {
            if inst.mode == OrderBookSymbolMode::Active {
                inst.set_book(OrderBook::empty());
                inst.asset_ctx = None;
                inst.spread_history.clear();
                inst.book_loading = true;
                inst.book_error = None;
            }
        }

        let mut candle_task = Task::none();
        if let Some(primary_id) = self.primary_chart_id {
            let mut old_cache_data = None;
            if let Some(instance) = self.charts.get(&primary_id)
                && !instance.chart.candles.is_empty()
                && matches!(instance.chart.status, ChartStatus::Loaded)
            {
                old_cache_data = Some((
                    instance.interval,
                    instance.symbol.clone(),
                    instance.chart.candles.clone(),
                ));
            }
            if let Some((old_tf, old_symbol, old_candles)) = old_cache_data {
                self.cache_candles(&old_symbol, old_tf, old_candles);
            }

            let mut cached_last_time = None;
            let target_interval = self
                .charts
                .get(&primary_id)
                .map(|inst| inst.interval)
                .unwrap_or(Timeframe::H1);
            let cached_candles = self.get_cached_candles(&valid_key, target_interval);

            let mut fetch_tf = None;
            if let Some(instance) = self.charts.get_mut(&primary_id) {
                instance.symbol = valid_key.clone();
                instance.symbol_display = display;
                instance.chart.request_view_reset();
                instance.heatmap_last_fetch = None;
                instance.heatmap_viewport = None;
                instance.heatmap_status = None;
                instance.heatmap_fetching = false;
                instance.last_price_flash = None;
                Self::clear_heatmap_display(instance);
                Self::clear_liquidation_display(instance);
                Self::clear_funding_display(instance);

                if let Some(candles) = cached_candles {
                    cached_last_time = candles.last().map(|c| c.open_time);
                    instance.chart.set_candles(candles);
                } else {
                    instance.chart.status = ChartStatus::Loading;
                    instance.chart.candles.clear();
                    instance.chart.candle_cache.clear();
                }

                instance.asset_ctx = None;
                instance.candle_fetch_error = None;
                fetch_tf = Some(instance.interval);
            }

            if let Some(tf) = fetch_tf {
                let mut tasks =
                    vec![self.queue_candle_fetch_for(primary_id, &valid_key, tf, cached_last_time)];
                tasks.extend(Self::fetch_macro_candles_tasks(primary_id, &valid_key));
                candle_task = Task::batch(tasks);
            }
        }

        self.sync_all_chart_overlays();
        for inst in self.charts.values_mut() {
            inst.clear_quick_order();
        }
        self.persist_config();

        let active_book_ids: Vec<_> = self
            .order_books
            .values()
            .filter(|b| b.mode == OrderBookSymbolMode::Active)
            .map(|b| b.id)
            .collect();
        let book_task = Task::batch(
            active_book_ids
                .into_iter()
                .map(|id| self.order_book_fetch_task_for_id(id)),
        );
        Task::batch([candle_task, book_task])
    }
}
