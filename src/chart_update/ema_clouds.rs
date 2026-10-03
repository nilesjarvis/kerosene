use crate::app_state::TradingTerminal;
use crate::chart_indicator::ChartIndicatorId;
use crate::chart_state::ChartId;
use crate::config::{EmaCloudConfig, EmaCloudPeriod, EmaCloudTimeframe, MAX_EMA_CLOUDS};
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_chart_ema_clouds(&mut self, message: Message) -> Task<Message> {
        let id = match &message {
            Message::ChartEmaCloudAdded(id)
            | Message::ChartEmaCloudRemoved(id, _)
            | Message::ChartEmaCloudToggled(id, _)
            | Message::ChartEmaCloudPeriodChanged(id, _, _, _)
            | Message::ChartEmaCloudTimeframeChanged(id, _, _)
            | Message::ChartEmaCloudColorChanged(id, _, _)
            | Message::ChartEmaCloudOpacityChanged(id, _, _) => *id,
            _ => return Task::none(),
        };
        let Some(instance) = self.charts.get_mut(&id) else {
            return Task::none();
        };
        let before = instance.macro_indicators.ema_clouds.clone();
        let clouds = &mut instance.macro_indicators.ema_clouds;
        let mut history_cloud_id = None;
        match message {
            Message::ChartEmaCloudAdded(_) => {
                if clouds.len() >= MAX_EMA_CLOUDS {
                    return Task::none();
                }
                let Some(cloud_id) = clouds
                    .iter()
                    .map(|cloud| cloud.id)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)
                else {
                    return Task::none();
                };
                clouds.push(EmaCloudConfig {
                    id: cloud_id,
                    ..Default::default()
                });
                history_cloud_id = Some(cloud_id);
            }
            Message::ChartEmaCloudRemoved(_, cloud_id) => {
                clouds.retain(|cloud| cloud.id != cloud_id);
                instance
                    .ema_cloud_period_inputs
                    .retain(|(id, _), _| *id != cloud_id);
            }
            Message::ChartEmaCloudToggled(_, cloud_id) => {
                if let Some(cloud) = clouds.iter_mut().find(|cloud| cloud.id == cloud_id) {
                    cloud.enabled = !cloud.enabled;
                    history_cloud_id = Some(cloud_id);
                }
            }
            Message::ChartEmaCloudPeriodChanged(_, cloud_id, field, value) => {
                let Some(cloud) = clouds.iter_mut().find(|cloud| cloud.id == cloud_id) else {
                    return Task::none();
                };
                if value.len() > 4 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Task::none();
                }
                let period = value.parse::<usize>().ok();
                if period.is_some_and(|period| period > ChartIndicatorId::MAX_MOVING_AVERAGE_PERIOD)
                {
                    return Task::none();
                }
                instance
                    .ema_cloud_period_inputs
                    .insert((cloud_id, field), value);
                let Some(period) = period.filter(|period| *period > 0) else {
                    return Task::none();
                };
                match field {
                    EmaCloudPeriod::Fast => cloud.fast_period = period,
                    EmaCloudPeriod::Slow => cloud.slow_period = period,
                }
                history_cloud_id = Some(cloud_id);
            }
            Message::ChartEmaCloudTimeframeChanged(_, cloud_id, timeframe) => {
                if let Some(cloud) = clouds.iter_mut().find(|cloud| cloud.id == cloud_id) {
                    cloud.timeframe = timeframe;
                    history_cloud_id = Some(cloud_id);
                }
            }
            Message::ChartEmaCloudColorChanged(_, cloud_id, color) => {
                if let Some(cloud) = clouds.iter_mut().find(|cloud| cloud.id == cloud_id) {
                    cloud.color = color;
                }
            }
            Message::ChartEmaCloudOpacityChanged(_, cloud_id, opacity) => {
                if let Some(cloud) = clouds.iter_mut().find(|cloud| cloud.id == cloud_id) {
                    cloud.opacity = opacity.min(100);
                }
            }
            _ => return Task::none(),
        }
        if *clouds == before {
            return Task::none();
        }
        instance.chart.macro_indicators = instance.macro_indicators.clone();
        instance.chart.candle_cache.clear();
        self.persist_config();
        history_cloud_id
            .map(|cloud_id| self.ensure_ema_cloud_history(id, cloud_id))
            .unwrap_or_else(Task::none)
    }

    fn ensure_ema_cloud_history(&mut self, id: ChartId, cloud_id: u64) -> Task<Message> {
        let Some(instance) = self.charts.get(&id) else {
            return Task::none();
        };
        let Some(cloud) = instance
            .macro_indicators
            .ema_clouds
            .iter()
            .find(|cloud| cloud.id == cloud_id)
        else {
            return Task::none();
        };
        if !cloud.enabled
            || instance.symbol.is_empty()
            || self.symbol_key_is_hidden(&instance.symbol)
        {
            return Task::none();
        }
        let symbol = instance.symbol.clone();
        if cloud.timeframe == EmaCloudTimeframe::Chart {
            if instance.chart.candles.len() >= cloud.fast_period.max(cloud.slow_period) {
                return Task::none();
            }
            self.queue_candle_fetch_for(id, &symbol, instance.interval, None)
        } else {
            Task::batch(self.queue_macro_candles_tasks(id, &symbol))
        }
    }
}

#[cfg(test)]
mod tests;
