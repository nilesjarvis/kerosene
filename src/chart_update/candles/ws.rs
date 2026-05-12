use crate::api::{Candle, is_valid_candle};
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
            let previous_close = instance.chart.candles.last().map(|candle| candle.close);
            let next_close = candle.close;
            let should_flash = is_valid_candle(&candle);
            instance.chart.push_candle(candle);
            if should_flash {
                instance.track_last_price_update(previous_close, next_close, Self::now_ms());
            }
            refresh_funding = instance.macro_indicators.show_funding_rate;
        }
        if refresh_funding {
            return self.maybe_fetch_chart_funding(id);
        }
        Task::none()
    }
}
