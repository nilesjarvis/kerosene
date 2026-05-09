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

        if self.is_ticker_muted(&key) {
            self.symbol_search_status = Some((format!("{key} is muted in Settings > Risk"), true));
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.editor_open = false;
                instance.editor_search_query.clear();
                instance.editor_keyboard_selected = false;
            }
            return Task::none();
        }

        let already_same = self.charts.get(&id).is_some_and(|inst| inst.symbol == key);
        if already_same {
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.editor_open = false;
                instance.editor_search_query.clear();
                instance.editor_keyboard_selected = false;
            }
            return Task::none();
        }

        if self.primary_chart_id == Some(id) {
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.annotations.clear();
                instance.next_annotation_id = 0;
                instance.chart.annotations.clear();
                instance.editor_open = false;
                instance.editor_search_query.clear();
                instance.editor_keyboard_selected = false;
                instance.liquidation_data = None;
                instance.heatmap_last_fetch = None;
                instance.heatmap_viewport = None;
                instance.heatmap_status = None;
                instance.heatmap_fetching = false;
                Self::clear_heatmap_display(instance);
            }

            let chase_cancel_task = if self.active_chase.is_some() {
                self.stop_chase()
            } else {
                Task::none()
            };

            let switch_task = self.switch_active_symbol_internal(key);
            return Task::batch([chase_cancel_task, switch_task]);
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

        let mut cached_last_time = None;
        let target_interval = self
            .charts
            .get(&id)
            .map(|inst| inst.interval)
            .unwrap_or(Timeframe::H1);
        let cached_candles = self.get_cached_candles(&key, target_interval);

        if let Some(instance) = self.charts.get_mut(&id) {
            let sym = self.exchange_symbols.iter().find(|s| s.key == key);
            let display = sym
                .map(Self::exchange_symbol_display_name)
                .unwrap_or_else(|| key.split(':').nth(1).unwrap_or(&key).to_string());
            instance.symbol = key.clone();
            instance.symbol_display = display;
            instance.chart.request_view_reset();

            if let Some(candles) = cached_candles {
                cached_last_time = candles.last().map(|c| c.open_time);
                instance.chart.set_candles(candles);
            } else {
                instance.chart.status = ChartStatus::Loading;
                instance.chart.candles.clear();
                instance.chart.candle_cache.clear();
            }

            instance.asset_ctx = None;
            instance.editor_open = false;
            instance.editor_search_query.clear();
            instance.editor_keyboard_selected = false;
            instance.liquidation_data = None;
            instance.heatmap_last_fetch = None;
            instance.heatmap_viewport = None;
            instance.heatmap_status = None;
            instance.heatmap_fetching = false;
            instance.candle_fetch_error = None;
            Self::clear_heatmap_display(instance);
            tf = instance.interval;
        }
        self.persist_config();
        let mut tasks = vec![self.queue_candle_fetch_for(id, &key, tf, cached_last_time)];
        tasks.extend(Self::fetch_macro_candles_tasks(id, &key));
        Task::batch(tasks)
    }
}
