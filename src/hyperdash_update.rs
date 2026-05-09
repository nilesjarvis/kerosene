use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod heatmap;
mod key;
mod liquidations;

impl TradingTerminal {
    pub(crate) fn update_hyperdash(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::HyperdashKeyInputChanged(_) | Message::SaveHyperdashKey) => {
                return self.update_hyperdash_key(message);
            }
            Message::ToggleLiquidationOverlay(chart_id) => {
                return self.toggle_liquidation_overlay(chart_id);
            }
            Message::ChartLiquidationLoaded(request_key, result) => {
                return self.apply_chart_liquidation_loaded(request_key, *result);
            }
            Message::RefreshLiquidations => return self.refresh_liquidations(),
            message @ (Message::ToggleHeatmapOverlay(_)
            | Message::ChartHeatmapLoaded(_, _)
            | Message::RefreshHeatmap) => {
                return self.update_hyperdash_heatmap(message);
            }
            _ => {}
        }

        Task::none()
    }
}
