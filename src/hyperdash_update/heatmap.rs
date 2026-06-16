use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_hyperdash_heatmap(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleHeatmapOverlay(chart_id) => self.toggle_heatmap_overlay(chart_id),
            Message::ChartHeatmapLoaded(cache_key, generation, result) => {
                self.apply_chart_heatmap_loaded(cache_key, generation, *result);
                Task::none()
            }
            Message::RefreshHeatmap => self.refresh_heatmap(),
            _ => Task::none(),
        }
    }

    fn toggle_heatmap_overlay(&mut self, chart_id: ChartId) -> Task<Message> {
        let hyperdash_key_missing = self.hyperdash_api_key.trim().is_empty();
        let chart_symbol = self
            .charts
            .get(&chart_id)
            .map(|instance| instance.symbol.clone())
            .unwrap_or_default();
        let chart_symbol_muted = self.symbol_key_is_hidden(&chart_symbol);
        let mut show_key_prompt = false;
        let should_fetch = if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.show_heatmap = !instance.show_heatmap;
            if !instance.show_heatmap {
                instance.heatmap_last_fetch = None;
                instance.heatmap_fetching = false;
                instance.heatmap_status = None;
                Self::clear_heatmap_display(instance);
                self.clear_chart_heatmap_pending_request_state(chart_id);
                false
            } else if hyperdash_key_missing {
                instance.heatmap_last_fetch = None;
                instance.heatmap_fetching = false;
                instance.heatmap_status = Some((
                    "Add HyperDash key in Settings > Integrations".to_string(),
                    true,
                ));
                Self::clear_heatmap_display(instance);
                show_key_prompt = true;
                false
            } else if chart_symbol_muted {
                instance.show_heatmap = false;
                instance.heatmap_status =
                    Some(("Ticker is hidden in Settings > Risk".to_string(), true));
                Self::clear_heatmap_display(instance);
                false
            } else {
                instance.heatmap_data.is_none() && !instance.symbol.is_empty()
            }
        } else {
            false
        };
        if show_key_prompt {
            self.push_toast(
                "Add a HyperDash API key in Settings > Integrations to load HEAT".to_string(),
                true,
            );
        }
        if should_fetch {
            return self.maybe_fetch_heatmap(chart_id);
        }

        Task::none()
    }

    fn apply_chart_heatmap_loaded(
        &mut self,
        cache_key: String,
        generation: u64,
        result: Result<crate::hyperdash_api::LiquidationHeatmap, String>,
    ) {
        if !self.hyperdash_key_generation_is_current(generation) {
            return;
        }

        let pending = self
            .heatmap_pending_charts
            .remove(&cache_key)
            .unwrap_or_default();
        if pending.is_empty() {
            return;
        }
        match result {
            Ok(data) => {
                self.cache_heatmap_data(cache_key.clone(), data.clone());
                for chart_id in pending {
                    self.apply_heatmap_data_to_chart(chart_id, &cache_key, &data, false);
                }
            }
            Err(e) => {
                let mut failed_visible_chart = false;
                for chart_id in pending {
                    if let Some(instance) = self.charts.get_mut(&chart_id) {
                        let requested_key = instance
                            .heatmap_last_fetch
                            .as_ref()
                            .map(crate::hyperdash_api::HeatmapFetchParams::cache_key);
                        if !instance.show_heatmap || requested_key.as_deref() != Some(&cache_key) {
                            continue;
                        }
                        instance.heatmap_fetching = false;
                        instance.heatmap_last_fetch = None;
                        instance.heatmap_status = Some(("HEAT fetch failed".to_string(), true));
                        Self::clear_heatmap_display(instance);
                        failed_visible_chart = true;
                    }
                }
                if failed_visible_chart {
                    self.push_toast(format!("Heatmap fetch failed: {e}"), true);
                }
            }
        }
    }

    fn refresh_heatmap(&mut self) -> Task<Message> {
        if self.hyperdash_api_key.is_empty() {
            return Task::none();
        }
        let ids: Vec<ChartId> = self
            .charts
            .iter()
            .filter(|(_, inst)| {
                inst.show_heatmap
                    && !inst.symbol.is_empty()
                    && !self.symbol_key_is_hidden(&inst.symbol)
            })
            .map(|(id, _)| *id)
            .collect();
        if ids.is_empty() {
            return Task::none();
        }
        let mut tasks = Vec::new();
        for id in ids {
            let task = self.maybe_fetch_heatmap(id);
            tasks.push(task);
        }
        Task::batch(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::chart_state::ChartInstance;
    use crate::hyperdash_api::{HeatmapFetchParams, LiquidationHeatmap};
    use crate::timeframe::Timeframe;

    #[test]
    fn stale_hyperdash_generation_heatmap_result_keeps_current_pending_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        let cache_key = "BTC:1.00000000:2.00000000:10:20".to_string();
        terminal.hyperdash_key_generation = 2;
        terminal
            .heatmap_pending_charts
            .insert(cache_key.clone(), vec![7]);

        terminal.apply_chart_heatmap_loaded(
            cache_key.clone(),
            1,
            Ok(LiquidationHeatmap {
                rects: Vec::new(),
                max_abs_usd: 0.0,
            }),
        );

        assert_eq!(
            terminal.heatmap_pending_charts.get(&cache_key),
            Some(&vec![7])
        );
    }

    #[test]
    fn heatmap_result_without_pending_charts_is_ignored() {
        let (mut terminal, _) = TradingTerminal::boot();
        let cache_key = "BTC:1.00000000:2.00000000:10:20".to_string();
        let generation = terminal.hyperdash_key_generation;

        terminal.apply_chart_heatmap_loaded(
            cache_key.clone(),
            generation,
            Ok(LiquidationHeatmap {
                rects: Vec::new(),
                max_abs_usd: 0.0,
            }),
        );
        terminal.apply_chart_heatmap_loaded(
            cache_key.clone(),
            generation,
            Err("late failure".to_string()),
        );

        assert!(!terminal.heatmap_data_cache.contains_key(&cache_key));
        assert!(terminal.toasts.is_empty());
    }

    #[test]
    fn disabling_heatmap_overlay_removes_pending_waiter_and_ignores_late_error() {
        let (mut terminal, _) = TradingTerminal::boot();
        let chart_id = 1;
        let cache_key = "BTC:1.00000000:2.00000000:10:20".to_string();
        let generation = terminal.hyperdash_key_generation;
        terminal.charts.clear();
        let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
        instance.show_heatmap = true;
        instance.heatmap_fetching = true;
        instance.heatmap_status = Some(("HEAT refreshing hourly data".to_string(), false));
        terminal.charts.insert(chart_id, instance);
        terminal
            .heatmap_pending_charts
            .insert(cache_key.clone(), vec![chart_id]);

        let _task = terminal.toggle_heatmap_overlay(chart_id);
        terminal.apply_chart_heatmap_loaded(
            cache_key.clone(),
            generation,
            Err("late failure".to_string()),
        );

        assert!(!terminal.heatmap_pending_charts.contains_key(&cache_key));
        assert!(terminal.toasts.is_empty());
        let instance = terminal.charts.get(&chart_id).expect("chart");
        assert!(!instance.show_heatmap);
        assert!(!instance.heatmap_fetching);
        assert!(instance.heatmap_status.is_none());
    }

    #[test]
    fn late_heatmap_error_for_old_request_does_not_clear_current_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        let chart_id = 1;
        let stale_request = HeatmapFetchParams {
            coin: "BTC".to_string(),
            min_price: 1.0,
            max_price: 2.0,
            start_time: 10,
            end_time: 20,
        };
        let current_request = HeatmapFetchParams {
            coin: "BTC".to_string(),
            min_price: 3.0,
            max_price: 4.0,
            start_time: 30,
            end_time: 40,
        };
        let stale_key = stale_request.cache_key();
        let current_key = current_request.cache_key();
        let generation = terminal.hyperdash_key_generation;
        terminal.charts.clear();
        let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
        instance.show_heatmap = true;
        instance.heatmap_fetching = true;
        instance.heatmap_last_fetch = Some(current_request);
        instance.heatmap_status = Some(("HEAT refreshing current data".to_string(), false));
        terminal.charts.insert(chart_id, instance);
        terminal
            .heatmap_pending_charts
            .insert(stale_key.clone(), vec![chart_id]);

        terminal.apply_chart_heatmap_loaded(stale_key, generation, Err("late failure".to_string()));

        assert!(terminal.toasts.is_empty());
        let instance = terminal.charts.get(&chart_id).expect("chart");
        assert!(instance.heatmap_fetching);
        assert_eq!(
            instance
                .heatmap_last_fetch
                .as_ref()
                .map(HeatmapFetchParams::cache_key)
                .as_deref(),
            Some(current_key.as_str())
        );
        assert_eq!(
            instance
                .heatmap_status
                .as_ref()
                .map(|(message, is_error)| { (message.as_str(), *is_error) }),
            Some(("HEAT refreshing current data", false))
        );
    }
}
