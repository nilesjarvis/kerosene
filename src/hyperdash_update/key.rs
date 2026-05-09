use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn update_hyperdash_key(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HyperdashKeyInputChanged(value) => {
                self.hyperdash_key_input.zeroize();
                self.hyperdash_key_input = value.into();
            }
            Message::SaveHyperdashKey => {
                self.hyperdash_api_key.zeroize();
                self.hyperdash_api_key = self.hyperdash_key_input.trim().to_string().into();
                self.persist_hyperdash_secret();
                self.persist_config();
                let ids: Vec<ChartId> = self
                    .charts
                    .iter()
                    .filter(|(_, inst)| inst.show_heatmap && !inst.symbol.is_empty())
                    .map(|(id, _)| *id)
                    .collect();
                if self.hyperdash_api_key.is_empty() {
                    for id in ids {
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
                    return Task::none();
                }
                let tasks: Vec<Task<Message>> = ids
                    .into_iter()
                    .map(|id| self.maybe_fetch_heatmap(id))
                    .collect();
                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }
            _ => {}
        }

        Task::none()
    }
}
