use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn update_hyperdash_key(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HyperdashKeyInputChanged(value) => {
                self.hyperdash_key_input.zeroize();
                self.hyperdash_key_input = value.into_zeroizing();
            }
            Message::SaveHyperdashKey => {
                self.hyperdash_api_key.zeroize();
                self.hyperdash_api_key = self.hyperdash_key_input.trim().to_string().into();
                if self.persist_hyperdash_secret() {
                    self.persist_config();
                }
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
                    self.heatmap_pending_charts.clear();
                    for id in heatmap_ids {
                        if let Some(instance) = self.charts.get_mut(&id) {
                            instance.heatmap_fetching = false;
                            instance.heatmap_last_fetch = None;
                            instance.heatmap_status = Some((
                                "Add HyperDash key in Settings > Integrations".to_string(),
                                true,
                            ));
                            Self::clear_heatmap_display(instance);
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
                    return Task::none();
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
                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }
            _ => {}
        }

        Task::none()
    }
}
