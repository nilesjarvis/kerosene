use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_hyperdash_heatmap(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleHeatmapOverlay(chart_id) => self.toggle_heatmap_overlay(chart_id),
            Message::ChartHeatmapLoaded(cache_key, result) => {
                self.apply_chart_heatmap_loaded(cache_key, *result);
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
        let chart_symbol_muted = self.is_ticker_muted(&chart_symbol);
        let mut show_key_prompt = false;
        let should_fetch = if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.show_heatmap = !instance.show_heatmap;
            if !instance.show_heatmap {
                instance.heatmap_last_fetch = None;
                instance.heatmap_fetching = false;
                instance.heatmap_status = None;
                Self::clear_heatmap_display(instance);
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
                    Some(("Ticker is muted in Settings > Risk".to_string(), true));
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
        result: Result<crate::hyperdash_api::LiquidationHeatmap, String>,
    ) {
        let pending = self
            .heatmap_pending_charts
            .remove(&cache_key)
            .unwrap_or_default();
        match result {
            Ok(data) => {
                self.cache_heatmap_data(cache_key.clone(), data.clone());
                for chart_id in pending {
                    self.apply_heatmap_data_to_chart(chart_id, &cache_key, &data, false);
                }
            }
            Err(e) => {
                for chart_id in pending {
                    if let Some(instance) = self.charts.get_mut(&chart_id) {
                        instance.heatmap_fetching = false;
                        instance.heatmap_last_fetch = None;
                        instance.heatmap_status = Some(("HEAT fetch failed".to_string(), true));
                        Self::clear_heatmap_display(instance);
                    }
                }
                self.push_toast(format!("Heatmap fetch failed: {e}"), true);
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
                inst.show_heatmap && !inst.symbol.is_empty() && !self.is_ticker_muted(&inst.symbol)
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
