use crate::api::{Candle, is_valid_candle};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartId;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn apply_chart_ws_candle_update(
        &mut self,
        _id: ChartId,
        symbol: String,
        interval: String,
        candle: Candle,
    ) -> Task<Message> {
        if self.symbol_key_is_hidden(&symbol) {
            return Task::none();
        }

        let now_ms = Self::now_ms();
        let should_flash = is_valid_candle(&candle);
        let mut refresh_funding_ids = Vec::new();

        for (chart_id, instance) in &mut self.charts {
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && instance.symbol == symbol
                && instance.interval.api_str() == interval
            {
                let previous_close = instance.chart.candles.last().map(|candle| candle.close);
                let next_close = candle.close;
                instance.chart.push_candle(candle.clone());
                if should_flash {
                    instance.track_last_price_update(previous_close, next_close, now_ms);
                }
                if instance.macro_indicators.show_funding_rate {
                    refresh_funding_ids.push(*chart_id);
                }
            }
        }

        if !refresh_funding_ids.is_empty() {
            return Task::batch(
                refresh_funding_ids
                    .into_iter()
                    .map(|chart_id| self.maybe_fetch_chart_funding(chart_id)),
            );
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
