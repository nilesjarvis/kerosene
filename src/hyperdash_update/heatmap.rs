use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::helpers::redact_sensitive_response_text;
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
            .map(|instance| instance.symbol.as_str())
            .unwrap_or_default();
        let chart_symbol_muted = self.symbol_key_is_hidden(chart_symbol);
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
                self.cache_heatmap_data(cache_key.clone(), data);
                for chart_id in pending {
                    self.apply_cached_heatmap_to_chart(chart_id, &cache_key, false);
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
                    self.push_toast(
                        format!(
                            "Heatmap fetch failed: {}",
                            redact_sensitive_response_text(&e)
                        ),
                        true,
                    );
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
        Task::batch(ids.into_iter().map(|id| self.maybe_fetch_heatmap(id)))
    }
}

#[cfg(test)]
mod tests;
