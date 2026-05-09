use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;

use iced::Task;

impl TradingTerminal {
    pub(super) fn update_pair_notional(
        &mut self,
        id: SpaghettiChartId,
        value: String,
    ) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.pair_notional = value;
        }
        Task::none()
    }

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

    pub(super) fn finish_pair_execution(
        &mut self,
        id: SpaghettiChartId,
        result: Result<String, String>,
    ) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.pair_pending = false;
        }
        match result {
            Ok(msg) => {
                self.order_status = Some((msg.clone(), false));
                self.push_toast(msg, false);
            }
            Err(e) => {
                self.order_status = Some((e.clone(), true));
                self.push_toast(e, true);
            }
        }
        Task::none()
    }
}
