use crate::app_state::TradingTerminal;
use crate::chart_state::ChartBackfillFetchContext;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::SpaghettiChartId;

use iced::Task;

impl TradingTerminal {
    pub(super) fn open_spaghetti_editor(&mut self, id: SpaghettiChartId) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.editor_open = true;
            inst.style_menu_open = false;
            inst.editor_search_query.clear();
        }
        Task::none()
    }

    pub(super) fn close_spaghetti_editor(&mut self, id: SpaghettiChartId) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.editor_open = false;
        }
        Task::none()
    }

    pub(super) fn update_spaghetti_editor_search(
        &mut self,
        id: SpaghettiChartId,
        query: String,
    ) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.editor_search_query = query;
        }
        Task::none()
    }

    pub(super) fn add_spaghetti_symbol(
        &mut self,
        id: SpaghettiChartId,
        key: String,
    ) -> Task<Message> {
        if self.symbol_key_is_hidden(&key) {
            self.push_toast("Ticker is hidden in Settings > Risk".to_string(), true);
            return Task::none();
        }
        let sym = self.exchange_symbols.iter().find(|s| s.key == key);
        if sym.is_some_and(|symbol| !symbol.is_user_selectable_market()) {
            self.push_toast("Market is not tradable".to_string(), true);
            return Task::none();
        }
        if let Some(preset_id) = self
            .spaghetti_charts
            .get(&id)
            .filter(|instance| !instance.pair_mode)
            .and_then(|instance| instance.watchlist_preset_id)
        {
            return self.add_watchlist_preset_symbol(preset_id, key);
        }
        self.add_spaghetti_symbol_direct(id, key)
    }

    fn add_spaghetti_symbol_direct(&mut self, id: SpaghettiChartId, key: String) -> Task<Message> {
        let sym = self.exchange_symbols.iter().find(|s| s.key == key);
        let display = sym
            .map(Self::exchange_symbol_display_name)
            .unwrap_or_else(|| key.split(':').nth(1).unwrap_or(&key).to_string());
        let theme = self.theme();
        let chart_backfill_source = self.chart_backfill_source;
        let read_data_provider_generation = self.read_data_provider_generation;
        let hydromancer_generation = self.hydromancer_key_generation;
        let hydromancer_api_key = self.hydromancer_api_key_for_task();
        let instance_epoch = self.spaghetti_instance_epoch;
        let mut task = Task::none();
        if let Some(inst) = self.spaghetti_charts.get_mut(&id)
            && !inst.canvas.series.iter().any(|s| s.symbol == key)
        {
            if inst.pair_mode && inst.canvas.series.len() >= 2 {
                self.push_toast(
                    "Pair ratio chart supports exactly two symbols".to_string(),
                    true,
                );
                return Task::none();
            }
            let color_idx = inst.next_color_idx;
            inst.next_color_idx += 1;
            let colors = spaghetti::series_colors(&theme);
            let color = colors[color_idx % colors.len()];
            inst.canvas.series.push(spaghetti::Series {
                symbol: key.clone(),
                display,
                candles: Vec::new(),
                color,
                loaded: false,
            });
            inst.canvas.apply_style_colors(&theme);
            task = Self::fetch_spaghetti_candles(
                id,
                instance_epoch,
                &key,
                inst.interval,
                inst.canvas.active_session,
                inst.session_granularity,
                ChartBackfillFetchContext::new(
                    chart_backfill_source,
                    read_data_provider_generation,
                    hydromancer_generation,
                    hydromancer_api_key,
                ),
            );
        }
        self.persist_config();
        task
    }

    pub(super) fn remove_spaghetti_symbol(
        &mut self,
        id: SpaghettiChartId,
        symbol: String,
    ) -> Task<Message> {
        if let Some(preset_id) = self
            .spaghetti_charts
            .get(&id)
            .filter(|instance| !instance.pair_mode)
            .and_then(|instance| instance.watchlist_preset_id)
        {
            return self.remove_watchlist_preset_symbol(preset_id, &symbol);
        }
        self.remove_spaghetti_symbol_direct(id, symbol)
    }

    fn remove_spaghetti_symbol_direct(
        &mut self,
        id: SpaghettiChartId,
        symbol: String,
    ) -> Task<Message> {
        let mut old_cache_data = None;
        if let Some(inst) = self.spaghetti_charts.get(&id) {
            let target_tf = Self::spaghetti_effective_timeframe_for(
                inst.interval,
                inst.canvas.active_session,
                inst.session_granularity,
                Self::now_ms(),
            );
            if let Some(series) = inst.canvas.series.iter().find(|s| s.symbol == symbol)
                && series.loaded
                && !series.candles.is_empty()
            {
                old_cache_data = Some((target_tf, symbol.clone(), series.candles.clone()));
            }
        }
        if let Some((tf, sym, candles)) = old_cache_data {
            self.cache_candles(&sym, tf, candles);
        }

        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.canvas.series.retain(|s| s.symbol != symbol);
            inst.health.remove(&symbol);
            Self::refresh_spaghetti_session_anchor(inst);
            inst.canvas.cache.clear();
            self.persist_config();
        }
        Task::none()
    }

    pub(crate) fn select_spaghetti_watchlist_preset(
        &mut self,
        id: SpaghettiChartId,
        preset_id: crate::config::WatchlistPresetId,
    ) -> Task<Message> {
        if self.watchlist_preset(preset_id).is_none() {
            return Task::none();
        }
        let Some(instance) = self.spaghetti_charts.get_mut(&id) else {
            return Task::none();
        };
        if instance.pair_mode {
            return Task::none();
        }
        instance.watchlist_preset_id = Some(preset_id);
        let task = self.sync_watchlist_preset_consumers(preset_id);
        self.persist_config();
        task
    }

    pub(crate) fn clear_spaghetti_watchlist_preset(
        &mut self,
        id: SpaghettiChartId,
    ) -> Task<Message> {
        if let Some(instance) = self.spaghetti_charts.get_mut(&id) {
            instance.watchlist_preset_id = None;
            self.persist_config();
        }
        Task::none()
    }

    pub(crate) fn sync_watchlist_preset_consumers(
        &mut self,
        preset_id: crate::config::WatchlistPresetId,
    ) -> Task<Message> {
        let Some(preset_symbols) = self
            .watchlist_preset(preset_id)
            .map(|preset| preset.symbols.clone())
        else {
            return Task::none();
        };
        let live_symbols = preset_symbols
            .iter()
            .filter(|symbol| !self.symbol_key_is_hidden(symbol))
            .cloned()
            .collect::<Vec<_>>();
        for watchlist in self.live_watchlists.values_mut() {
            if watchlist.preset_id == Some(preset_id) {
                watchlist.symbols.clone_from(&live_symbols);
            }
        }
        self.refresh_live_watchlist_row_caches();

        let chart_symbols = preset_symbols
            .into_iter()
            .filter(|symbol| !self.symbol_key_is_hidden(symbol))
            .filter(|symbol| {
                self.exchange_symbols
                    .iter()
                    .find(|candidate| candidate.key == *symbol)
                    .is_none_or(|candidate| candidate.is_user_selectable_market())
            })
            .collect::<Vec<_>>();
        let chart_ids = self
            .spaghetti_charts
            .iter()
            .filter_map(|(id, instance)| {
                (!instance.pair_mode && instance.watchlist_preset_id == Some(preset_id))
                    .then_some(*id)
            })
            .collect::<Vec<_>>();
        let mut tasks = Vec::new();
        for chart_id in chart_ids {
            let existing = self.spaghetti_charts[&chart_id]
                .canvas
                .series
                .iter()
                .map(|series| series.symbol.clone())
                .collect::<Vec<_>>();
            for symbol in existing
                .iter()
                .filter(|symbol| !chart_symbols.contains(symbol))
            {
                tasks.push(self.remove_spaghetti_symbol_direct(chart_id, symbol.clone()));
            }
            for symbol in chart_symbols
                .iter()
                .filter(|symbol| !existing.contains(symbol))
            {
                tasks.push(self.add_spaghetti_symbol_direct(chart_id, symbol.clone()));
            }
            if let Some(instance) = self.spaghetti_charts.get_mut(&chart_id) {
                instance.canvas.series.sort_by_key(|series| {
                    chart_symbols
                        .iter()
                        .position(|symbol| symbol == &series.symbol)
                        .unwrap_or(usize::MAX)
                });
                instance.canvas.cache.clear();
            }
        }
        Task::batch(tasks)
    }

    pub(crate) fn sync_all_watchlist_preset_consumers(&mut self) -> Task<Message> {
        let preset_ids = self
            .watchlist_presets
            .iter()
            .map(|preset| preset.id)
            .collect::<Vec<_>>();
        Task::batch(
            preset_ids
                .into_iter()
                .map(|preset_id| self.sync_watchlist_preset_consumers(preset_id)),
        )
    }
}
