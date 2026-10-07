use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use zeroize::{Zeroize, Zeroizing};

impl TradingTerminal {
    pub(super) fn update_hyperdash_key(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HyperdashKeyInputChanged(value) => {
                self.hyperdash_key_input.zeroize();
                self.hyperdash_key_input = value.into_zeroizing().into();
            }
            Message::SaveHyperdashKey => {
                let previous_key = Zeroizing::new(self.hyperdash_api_key.trim().to_string());
                let next_key = Zeroizing::new(self.hyperdash_key_input.trim().to_string());
                if !self.persist_hyperdash_secret_from_key(next_key.as_str()) {
                    return Task::none();
                }

                self.hyperdash_api_key.zeroize();
                self.hyperdash_api_key = next_key.as_str().to_string().into();
                let hyperdash_key_changed = previous_key.as_str() != next_key.as_str();
                if hyperdash_key_changed {
                    self.bump_hyperdash_key_generation();
                    // Pi captures the key before it reports Ready. Invalidate
                    // starting runtimes and pending snapshots as well as live ones.
                    self.invalidate_agent_runtime();
                }
                self.persist_config();
                let heatmap_ids: Vec<ChartId> = self
                    .charts
                    .iter()
                    .filter(|(_, inst)| inst.show_heatmap && !inst.symbol.is_empty())
                    .map(|(id, _)| *id)
                    .collect();
                let liquidation_ids: Vec<ChartId> = self
                    .charts
                    .iter()
                    .filter(|(_, inst)| inst.show_liquidations && !inst.symbol.is_empty())
                    .map(|(id, _)| *id)
                    .collect();
                let distribution_open =
                    self.pane_is_open(|kind| matches!(kind, PaneKind::LiquidationsDistribution));
                self.liquidation_pending_charts.clear();
                for id in &liquidation_ids {
                    if let Some(instance) = self.charts.get_mut(id) {
                        instance.liquidation_fetching = false;
                        instance.liquidation_pending_key = None;
                    }
                }
                if self.hyperdash_api_key.is_empty() {
                    for id in heatmap_ids {
                        if let Some(instance) = self.charts.get_mut(&id) {
                            instance.heatmap_status = Some((
                                "Add HyperDash key in Settings > Integrations".to_string(),
                                true,
                            ));
                        }
                    }
                    for id in liquidation_ids {
                        if let Some(instance) = self.charts.get_mut(&id) {
                            Self::clear_liquidation_display(instance);
                            instance.liquidation_status = Some((
                                "Add HyperDash key in Settings > Integrations".to_string(),
                                true,
                            ));
                            instance.chart.candle_cache.clear();
                        }
                    }
                    if distribution_open {
                        let _ = self.request_liquidation_distribution_refresh(true);
                    }
                    return self.request_positioning_info_refresh_all(true);
                }
                let mut tasks: Vec<Task<Message>> = heatmap_ids
                    .into_iter()
                    .map(|id| self.maybe_fetch_heatmap(id))
                    .collect();
                tasks.extend(
                    liquidation_ids
                        .into_iter()
                        .map(|id| self.maybe_fetch_liquidations(id)),
                );
                if distribution_open {
                    tasks.push(self.request_liquidation_distribution_refresh(true));
                }
                tasks.push(self.request_positioning_info_refresh_all(true));
                return Task::batch(tasks);
            }
            _ => {}
        }

        Task::none()
    }

    pub(crate) fn invalidate_hyperdash_chart_requests_for_key_change(&mut self) {
        self.heatmap_pending_charts.clear();
        self.heatmap_data_cache.clear();
        self.heatmap_data_cache_order.clear();

        for instance in self.charts.values_mut().filter(|instance| {
            instance.show_heatmap
                || instance.heatmap_fetching
                || instance.heatmap_last_fetch.is_some()
                || instance.heatmap_data.is_some()
        }) {
            instance.heatmap_fetching = false;
            instance.heatmap_last_fetch = None;
            instance.heatmap_status = None;
            Self::clear_heatmap_display(instance);
        }

        self.liquidation_pending_charts.clear();
        for instance in self.charts.values_mut() {
            instance.liquidation_fetching = false;
            instance.liquidation_pending_key = None;
        }
    }
}

#[cfg(test)]
mod tests;
