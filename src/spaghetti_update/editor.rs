use crate::app_state::TradingTerminal;
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
        if self.is_ticker_muted(&key) {
            self.push_toast("Ticker is muted in Settings > Risk".to_string(), true);
            return Task::none();
        }
        let sym = self.exchange_symbols.iter().find(|s| s.key == key);
        let display = sym
            .map(Self::exchange_symbol_display_name)
            .unwrap_or_else(|| key.split(':').nth(1).unwrap_or(&key).to_string());
        let theme = self.theme();
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
                &key,
                inst.interval,
                inst.canvas.active_session,
                inst.session_granularity,
                None,
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
            Self::refresh_spaghetti_session_anchor(inst);
            inst.canvas.cache.clear();
            self.persist_config();
        }
        Task::none()
    }
}
