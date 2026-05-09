use crate::api::Candle;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn apply_chart_ws_candle_update(
        &mut self,
        id: ChartId,
        symbol: String,
        interval: String,
        candle: Candle,
    ) -> Task<Message> {
        if self.is_ticker_muted(&symbol) {
            return Task::none();
        }
        let mut refresh_funding = false;
        if let Some(instance) = self.charts.get_mut(&id)
            && instance.symbol == symbol
            && instance.interval.api_str() == interval
        {
            instance.chart.push_candle(candle);
            refresh_funding = instance.macro_indicators.show_funding_rate;
        }
        if refresh_funding {
            return self.maybe_fetch_chart_funding(id);
        }
        Task::none()
    }
}
