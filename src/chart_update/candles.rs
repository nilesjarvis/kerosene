mod loaded;
mod reload;
mod timeframe;
mod ws;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(super) fn update_chart_candles(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartReload(id) => self.reload_chart_candles(id),
            Message::ChartSwitchTimeframe(id, timeframe) => {
                self.switch_chart_candle_timeframe(id, timeframe)
            }
            Message::ChartCandlesLoaded(request, result) => {
                self.apply_chart_candles_loaded(request, result)
            }
            Message::ChartWsCandleUpdate(id, symbol, interval, candle) => {
                self.apply_chart_ws_candle_update(id, symbol, interval, candle)
            }
            _ => Task::none(),
        }
    }
}
