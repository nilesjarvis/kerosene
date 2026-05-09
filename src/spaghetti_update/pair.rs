use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;

use iced::Task;

impl TradingTerminal {
    pub(super) fn set_pair_candle_mode(
        &mut self,
        id: SpaghettiChartId,
        enabled: bool,
    ) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.pair_candle_mode = enabled;
            inst.canvas.pair_candle_mode = enabled;
            inst.canvas.cache.clear();
            self.persist_config();
        }
        Task::none()
    }
}
