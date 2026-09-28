use crate::app_state::TradingTerminal;
use crate::config::{WatchlistPresetConfig, WatchlistPresetId};
use crate::market_state::LiveWatchlistId;
use crate::message::Message;

use iced::Task;
use std::collections::HashMap;

impl TradingTerminal {
    pub(crate) fn watchlist_preset(
        &self,
        preset_id: WatchlistPresetId,
    ) -> Option<&WatchlistPresetConfig> {
        self.watchlist_presets
            .iter()
            .find(|preset| preset.id == preset_id)
    }

    pub(crate) fn watchlist_preset_name(&self, preset_id: WatchlistPresetId) -> Option<&str> {
        self.watchlist_preset(preset_id)
            .map(WatchlistPresetConfig::display_name)
    }

    pub(crate) fn ensure_default_watchlist_preset(&mut self) -> WatchlistPresetId {
        if let Some(preset) = self.watchlist_presets.first() {
            return preset.id;
        }
        self.create_watchlist_preset(Vec::new())
    }

    pub(crate) fn resolve_watchlist_preset(
        &mut self,
        preset_id: Option<WatchlistPresetId>,
        fallback_symbols: &[String],
    ) -> WatchlistPresetId {
        if let Some(preset_id) = preset_id
            && self.watchlist_preset(preset_id).is_some()
        {
            return preset_id;
        }
        if let Some(preset) = self
            .watchlist_presets
            .iter()
            .find(|preset| preset.symbols == fallback_symbols)
        {
            return preset.id;
        }
        self.create_watchlist_preset(fallback_symbols.to_vec())
    }

    pub(crate) fn resolve_layout_watchlist_presets(
        &mut self,
        layout: &mut crate::config::SavedLayout,
    ) {
        let mut remapped_ids = HashMap::new();
        for watchlist in &mut layout.live_watchlists {
            let requested_id = watchlist.preset_id;
            let preset_id = requested_id
                .and_then(|id| remapped_ids.get(&id).copied())
                .unwrap_or_else(|| self.resolve_watchlist_preset(requested_id, &watchlist.symbols));
            if let Some(requested_id) = requested_id {
                remapped_ids.insert(requested_id, preset_id);
            }
            watchlist.preset_id = Some(preset_id);
            if let Some(preset) = self.watchlist_preset(preset_id) {
                watchlist.symbols = preset.symbols.clone();
            }
        }
        for chart in &mut layout.spaghetti_charts {
            if chart.pair_mode {
                chart.watchlist_preset_id = None;
                continue;
            }
            let Some(requested_id) = chart.watchlist_preset_id else {
                continue;
            };
            let preset_id = remapped_ids.get(&requested_id).copied().unwrap_or_else(|| {
                self.resolve_watchlist_preset(Some(requested_id), &chart.symbols)
            });
            chart.watchlist_preset_id = Some(preset_id);
            if let Some(preset) = self.watchlist_preset(preset_id) {
                chart.symbols = preset.symbols.clone();
            }
        }
    }

    pub(crate) fn create_watchlist_preset(&mut self, symbols: Vec<String>) -> WatchlistPresetId {
        let next_after_existing = self
            .watchlist_presets
            .iter()
            .map(|preset| preset.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let id = crate::ws::now_ms().max(next_after_existing);
        let ordinal = self.watchlist_presets.len().saturating_add(1);
        let base_name = format!("Watchlist {ordinal}");
        let name = self.unique_watchlist_preset_name(&base_name);
        self.watchlist_presets
            .push(WatchlistPresetConfig { id, name, symbols });
        id
    }

    fn unique_watchlist_preset_name(&self, preferred: &str) -> String {
        let base = if preferred.trim().is_empty() {
            "Untitled Watchlist".to_string()
        } else {
            preferred.trim().to_string()
        };
        let mut candidate = base.clone();
        let mut suffix = 2;
        while self
            .watchlist_presets
            .iter()
            .any(|preset| preset.name.eq_ignore_ascii_case(&candidate))
        {
            candidate = format!("{base} {suffix}");
            suffix += 1;
        }
        candidate
    }

    pub(crate) fn select_live_watchlist_preset(
        &mut self,
        watchlist_id: LiveWatchlistId,
        preset_id: WatchlistPresetId,
    ) -> Task<Message> {
        if !self.live_watchlists.contains_key(&watchlist_id) {
            return Task::none();
        }
        let Some(symbols) = self.watchlist_preset(preset_id).map(|preset| {
            preset
                .symbols
                .iter()
                .filter(|symbol| !self.symbol_key_is_hidden(symbol))
                .cloned()
                .collect::<Vec<_>>()
        }) else {
            return Task::none();
        };
        if let Some(watchlist) = self.live_watchlists.get_mut(&watchlist_id) {
            watchlist.preset_id = Some(preset_id);
            watchlist.symbols = symbols;
            watchlist.search_query.clear();
        }
        self.refresh_live_watchlist_row_cache(watchlist_id);
        self.persist_config();
        self.request_live_watchlist_refresh(true)
    }

    pub(crate) fn create_live_watchlist_preset(
        &mut self,
        watchlist_id: LiveWatchlistId,
    ) -> Task<Message> {
        if !self.live_watchlists.contains_key(&watchlist_id) {
            return Task::none();
        }
        let preset_id = self.create_watchlist_preset(Vec::new());
        self.select_live_watchlist_preset(watchlist_id, preset_id)
    }

    pub(crate) fn rename_watchlist_preset(
        &mut self,
        preset_id: WatchlistPresetId,
        name: String,
    ) -> Task<Message> {
        if let Some(preset) = self
            .watchlist_presets
            .iter_mut()
            .find(|preset| preset.id == preset_id)
        {
            preset.name = name;
            self.persist_config();
        }
        Task::none()
    }

    pub(crate) fn delete_watchlist_preset(
        &mut self,
        preset_id: WatchlistPresetId,
    ) -> Task<Message> {
        if self.watchlist_preset(preset_id).is_none() {
            return Task::none();
        }
        self.watchlist_presets
            .retain(|preset| preset.id != preset_id);
        let fallback_id = self.ensure_default_watchlist_preset();
        let fallback_symbols = self
            .watchlist_preset(fallback_id)
            .map(|preset| preset.symbols.clone())
            .unwrap_or_default();
        for watchlist in self.live_watchlists.values_mut() {
            if watchlist.preset_id == Some(preset_id) {
                watchlist.preset_id = Some(fallback_id);
            }
        }
        for chart in self.spaghetti_charts.values_mut() {
            if chart.watchlist_preset_id == Some(preset_id) {
                chart.watchlist_preset_id = Some(fallback_id);
            }
        }
        for layout in &mut self.saved_layouts {
            for watchlist in &mut layout.live_watchlists {
                if watchlist.preset_id == Some(preset_id) {
                    watchlist.preset_id = Some(fallback_id);
                    watchlist.symbols.clone_from(&fallback_symbols);
                }
            }
            for chart in &mut layout.spaghetti_charts {
                if chart.watchlist_preset_id == Some(preset_id) {
                    chart.watchlist_preset_id = Some(fallback_id);
                    chart.symbols.clone_from(&fallback_symbols);
                }
            }
        }
        let task = self.sync_watchlist_preset_consumers(fallback_id);
        self.persist_config();
        Task::batch([task, self.request_live_watchlist_refresh(true)])
    }

    pub(crate) fn add_watchlist_preset_symbol(
        &mut self,
        preset_id: WatchlistPresetId,
        symbol: String,
    ) -> Task<Message> {
        let mut changed = false;
        if let Some(preset) = self
            .watchlist_presets
            .iter_mut()
            .find(|preset| preset.id == preset_id)
            && !preset.symbols.contains(&symbol)
        {
            preset.symbols.push(symbol);
            changed = true;
        }
        if !changed {
            return Task::none();
        }
        self.sync_saved_layout_watchlist_preset_snapshots(preset_id);
        let task = self.sync_watchlist_preset_consumers(preset_id);
        self.persist_config();
        Task::batch([task, self.request_live_watchlist_refresh(true)])
    }

    pub(crate) fn remove_watchlist_preset_symbol(
        &mut self,
        preset_id: WatchlistPresetId,
        symbol: &str,
    ) -> Task<Message> {
        let mut changed = false;
        if let Some(preset) = self
            .watchlist_presets
            .iter_mut()
            .find(|preset| preset.id == preset_id)
        {
            let old_len = preset.symbols.len();
            preset.symbols.retain(|item| item != symbol);
            changed = old_len != preset.symbols.len();
        }
        if !changed {
            return Task::none();
        }
        self.sync_saved_layout_watchlist_preset_snapshots(preset_id);
        let task = self.sync_watchlist_preset_consumers(preset_id);
        self.persist_config();
        Task::batch([task, self.request_live_watchlist_refresh(true)])
    }

    pub(crate) fn sync_saved_layout_watchlist_preset_snapshots(
        &mut self,
        preset_id: WatchlistPresetId,
    ) {
        let Some(symbols) = self
            .watchlist_preset(preset_id)
            .map(|preset| preset.symbols.clone())
        else {
            return;
        };
        for layout in &mut self.saved_layouts {
            for watchlist in &mut layout.live_watchlists {
                if watchlist.preset_id == Some(preset_id) {
                    watchlist.symbols.clone_from(&symbols);
                }
            }
            for chart in &mut layout.spaghetti_charts {
                if chart.watchlist_preset_id == Some(preset_id) {
                    chart.symbols.clone_from(&symbols);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_state::LiveWatchlistInstance;
    use crate::spaghetti_state::SpaghettiChartInstance;

    fn live_watchlist(id: u64, preset_id: u64, symbols: &[&str]) -> LiveWatchlistInstance {
        LiveWatchlistInstance {
            id,
            preset_id: Some(preset_id),
            symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
            search_query: String::new(),
            sort_column: Default::default(),
            sort_direction: Default::default(),
            visible_columns: crate::config::default_live_watchlist_columns(),
            ema: Default::default(),
            ema_period_input: "20".to_string(),
            row_cache: Vec::new(),
        }
    }

    #[test]
    fn preset_symbol_changes_sync_every_linked_widget() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.watchlist_presets = vec![WatchlistPresetConfig {
            id: 44,
            name: "Majors".to_string(),
            symbols: vec!["BTC".to_string(), "ETH".to_string()],
        }];
        terminal.live_watchlists.clear();
        terminal
            .live_watchlists
            .insert(1, live_watchlist(1, 44, &[]));
        terminal
            .live_watchlists
            .insert(2, live_watchlist(2, 44, &["SOL"]));
        terminal.spaghetti_charts.clear();
        let mut chart = SpaghettiChartInstance::new_empty(7);
        chart.watchlist_preset_id = Some(44);
        terminal.spaghetti_charts.insert(7, chart);

        let _task = terminal.sync_watchlist_preset_consumers(44);

        assert_eq!(terminal.live_watchlists[&1].symbols, ["BTC", "ETH"]);
        assert_eq!(terminal.live_watchlists[&2].symbols, ["BTC", "ETH"]);
        assert_eq!(
            terminal.spaghetti_charts[&7]
                .canvas
                .series
                .iter()
                .map(|series| series.symbol.as_str())
                .collect::<Vec<_>>(),
            ["BTC", "ETH"]
        );

        let _task = terminal.remove_watchlist_preset_symbol(44, "BTC");

        assert_eq!(terminal.watchlist_presets[0].symbols, ["ETH"]);
        assert_eq!(terminal.live_watchlists[&1].symbols, ["ETH"]);
        assert_eq!(terminal.spaghetti_charts[&7].canvas.series[0].symbol, "ETH");
    }

    #[test]
    fn selecting_preset_replaces_comparison_symbols_and_unlink_keeps_them() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.watchlist_presets = vec![WatchlistPresetConfig {
            id: 12,
            name: "Layer Ones".to_string(),
            symbols: vec!["BTC".to_string(), "SOL".to_string()],
        }];
        terminal.spaghetti_charts.clear();
        terminal
            .spaghetti_charts
            .insert(5, SpaghettiChartInstance::new_empty(5));
        let _task = terminal.update_spaghetti(Message::SpaghettiAddSymbol(5, "ETH".to_string()));

        let _task = terminal.select_spaghetti_watchlist_preset(5, 12);

        let chart = &terminal.spaghetti_charts[&5];
        assert_eq!(chart.watchlist_preset_id, Some(12));
        assert_eq!(
            chart
                .canvas
                .series
                .iter()
                .map(|series| series.symbol.as_str())
                .collect::<Vec<_>>(),
            ["BTC", "SOL"]
        );

        let _task = terminal.clear_spaghetti_watchlist_preset(5);
        assert_eq!(terminal.spaghetti_charts[&5].watchlist_preset_id, None);
        assert_eq!(terminal.spaghetti_charts[&5].canvas.series.len(), 2);
    }

    #[test]
    fn legacy_layout_watchlist_and_linked_chart_resolve_to_one_global_preset() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.watchlist_presets.clear();
        let mut layout: crate::config::SavedLayout = serde_json::from_value(serde_json::json!({
            "name": "Imported",
            "live_watchlists": [{ "id": 2, "symbols": ["BTC", "ETH"] }],
            "spaghetti_charts": [{
                "id": 3,
                "symbols": ["BTC", "ETH"],
                "watchlist_preset_id": 999
            }]
        }))
        .expect("legacy layout");

        terminal.resolve_layout_watchlist_presets(&mut layout);

        assert_eq!(terminal.watchlist_presets.len(), 1);
        let preset_id = terminal.watchlist_presets[0].id;
        assert_eq!(layout.live_watchlists[0].preset_id, Some(preset_id));
        assert_eq!(
            layout.spaghetti_charts[0].watchlist_preset_id,
            Some(preset_id)
        );
    }
}
