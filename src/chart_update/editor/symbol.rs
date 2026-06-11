use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::message::Message;
use crate::timeframe::Timeframe;
use iced::Task;

impl TradingTerminal {
    pub(super) fn select_chart_symbol(&mut self, message: Message) -> Task<Message> {
        let Message::ChartSymbolSelected(id, key) = message else {
            return Task::none();
        };

        if self.symbol_key_is_hidden(&key) {
            let display = self.display_name_for_symbol(&key);
            self.symbol_search_status =
                Some((format!("{display} is hidden in Settings > Risk"), true));
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.editor_open = false;
                instance.editor_search_query.clear();
                instance.editor_selected_index = None;
            }
            return Task::none();
        }

        if self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == key)
            .is_some_and(|symbol| !symbol.is_user_selectable_market())
        {
            let display = self.display_name_for_symbol(&key);
            self.symbol_search_status = Some((format!("{display} is not a tradable market"), true));
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.editor_open = false;
                instance.editor_search_query.clear();
                instance.editor_selected_index = None;
            }
            return Task::none();
        }

        let already_same = self.charts.get(&id).is_some_and(|inst| inst.symbol == key);
        if already_same {
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.editor_open = false;
                instance.editor_search_query.clear();
                instance.editor_selected_index = None;
            }
            return Task::none();
        }

        if self.primary_chart_id == Some(id) {
            self.clear_all_chart_surface_state(id);
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.annotations.clear();
                instance.next_annotation_id = 0;
                instance.chart.annotations.clear();
                instance.editor_open = false;
                instance.editor_search_query.clear();
                instance.editor_selected_index = None;
                instance.heatmap_last_fetch = None;
                instance.heatmap_viewport = None;
                instance.heatmap_status = None;
                instance.heatmap_fetching = false;
                instance.last_price_flash = None;
                Self::clear_heatmap_display(instance);
                Self::clear_liquidation_display(instance);
                Self::clear_funding_display(instance);
                Self::clear_earnings_display(instance);
            }

            let switch_task = self.switch_active_symbol_internal(key);
            return switch_task;
        }

        let mut tf = Timeframe::H1;
        let mut old_cache_data = None;
        if let Some(instance) = self.charts.get(&id)
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
        self.clear_all_chart_surface_state(id);

        let mut cached_last_time = None;
        let target_interval = self
            .charts
            .get(&id)
            .map(|inst| inst.interval)
            .unwrap_or(Timeframe::H1);
        let cached_candles = self.get_cached_candles(&key, target_interval);
        let whole_unit_volume = self.is_outcome_coin(&key);

        if let Some(instance) = self.charts.get_mut(&id) {
            let sym = self.exchange_symbols.iter().find(|s| s.key == key);
            let display = sym
                .map(Self::exchange_symbol_display_name)
                .unwrap_or_else(|| key.split(':').nth(1).unwrap_or(&key).to_string());
            instance.symbol = key.clone();
            instance.symbol_display = display.clone();
            instance.chart.whole_unit_volume = whole_unit_volume;
            instance.chart.set_symbol_label(display);
            instance.chart.request_view_reset();
            instance.chart.clear_hud_armed();
            instance.chart.clear_macro_candles();

            if let Some(candles) = cached_candles {
                cached_last_time = candles.last().map(|c| c.open_time);
                instance.chart.set_candles(candles);
            } else {
                instance.chart.status = ChartStatus::Loading;
                instance.chart.candles.clear();
                instance.chart.candle_cache.clear();
            }

            instance.set_asset_context(None);
            instance.editor_open = false;
            instance.editor_search_query.clear();
            instance.editor_selected_index = None;
            instance.heatmap_last_fetch = None;
            instance.heatmap_viewport = None;
            instance.heatmap_status = None;
            instance.heatmap_fetching = false;
            instance.candle_fetch_error = None;
            instance.last_price_flash = None;
            Self::clear_heatmap_display(instance);
            Self::clear_liquidation_display(instance);
            Self::clear_funding_display(instance);
            Self::clear_earnings_display(instance);
            tf = instance.interval;
        }
        self.sync_chart_position_for(id);
        self.sync_chart_orders_for(id);
        self.sync_chart_trade_markers_for(id);
        self.sync_chart_market_reference_prices();
        self.persist_config();
        let mut tasks = vec![self.queue_candle_fetch_for(id, &key, tf, cached_last_time)];
        tasks.extend(Self::fetch_macro_candles_tasks(id, &key));
        if self
            .charts
            .get(&id)
            .is_some_and(|instance| instance.show_earnings_markers)
        {
            tasks.push(self.maybe_fetch_chart_earnings(id));
        }
        Task::batch(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
    use crate::chart_state::ChartInstance;
    use crate::config::ChartCrosshairStyle;

    fn fallback_outcome_symbol(key: &str) -> ExchangeSymbol {
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
                is_question_fallback: true,
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
    fn rejected_symbol_status_uses_outcome_display_name() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.primary_chart_id = None;
        terminal
            .exchange_symbols
            .push(fallback_outcome_symbol("#950"));

        let _task =
            terminal.select_chart_symbol(Message::ChartSymbolSelected(1, "#950".to_string()));

        let (status, is_error) = terminal.symbol_search_status.clone().expect("status set");
        assert!(is_error);
        assert!(status.contains("Will BTC close green?"), "{status}");
        assert!(!status.contains("#950"), "{status}");
    }

    #[test]
    fn selecting_chart_symbol_disarms_hud_trading() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.primary_chart_id = None;
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        instance.chart.set_hud_armed_at(true, 1_000);
        terminal.charts.insert(1, instance);

        let _task =
            terminal.select_chart_symbol(Message::ChartSymbolSelected(1, "ETH".to_string()));

        assert!(!terminal.charts.get(&1).unwrap().chart.hud_armed());
    }
}
