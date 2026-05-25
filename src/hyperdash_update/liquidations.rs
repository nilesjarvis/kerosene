use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use iced::Task;
use planning::{
    LiquidationPlanContext, LiquidationRequestPlan, liquidation_mark_from_ctx,
    liquidation_request_plan,
};

const LIQUIDATION_BUCKET_COUNT: usize = 200;

mod apply;
mod display;
mod planning;
mod requests;

impl TradingTerminal {
    pub(in crate::hyperdash_update) fn toggle_liquidation_overlay(
        &mut self,
        chart_id: ChartId,
    ) -> Task<Message> {
        let Some(is_enabled) = self
            .charts
            .get(&chart_id)
            .map(|instance| instance.show_liquidations)
        else {
            return Task::none();
        };

        if is_enabled {
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.show_liquidations = false;
                Self::clear_liquidation_display(instance);
            }
            return Task::none();
        }

        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.show_liquidations = true;
        }
        let plan = self
            .charts
            .get(&chart_id)
            .map(|instance| self.plan_liquidation_fetch_for_instance(instance))
            .unwrap_or(LiquidationRequestPlan::Wait);

        match plan {
            LiquidationRequestPlan::Fetch { coin, mark } => {
                self.queue_liquidation_fetch(chart_id, coin, mark)
            }
            LiquidationRequestPlan::Status(message, is_error) => {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.liquidation_status = Some((message.clone(), is_error));
                    instance.chart.candle_cache.clear();
                }
                if is_error {
                    self.push_toast(message, true);
                }
                Task::none()
            }
            LiquidationRequestPlan::Wait => Task::none(),
        }
    }

    pub(in crate::hyperdash_update) fn refresh_liquidations(&mut self) -> Task<Message> {
        if self.hyperdash_api_key.is_empty() {
            return Task::none();
        }
        let plans: Vec<(ChartId, String, f64)> = self
            .charts
            .values()
            .filter(|instance| instance.show_liquidations)
            .filter_map(|instance| {
                let LiquidationRequestPlan::Fetch { coin, mark } =
                    self.plan_liquidation_fetch_for_instance(instance)
                else {
                    return None;
                };
                Some((instance.id, coin, mark))
            })
            .collect();
        let now_secs = Self::now_ms() / 1_000;
        let tasks: Vec<Task<Message>> = plans
            .into_iter()
            .map(|(id, coin, mark)| self.queue_liquidation_fetch_at(id, coin, mark, now_secs))
            .collect();
        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    pub(crate) fn maybe_fetch_liquidations(&mut self, chart_id: ChartId) -> Task<Message> {
        let plan = self
            .charts
            .get(&chart_id)
            .map(|instance| self.plan_liquidation_fetch_for_instance(instance))
            .unwrap_or(LiquidationRequestPlan::Wait);
        match plan {
            LiquidationRequestPlan::Fetch { coin, mark } => {
                self.queue_liquidation_fetch(chart_id, coin, mark)
            }
            LiquidationRequestPlan::Status(message, is_error) => {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.liquidation_status = Some((message, is_error));
                    instance.chart.candle_cache.clear();
                }
                Task::none()
            }
            LiquidationRequestPlan::Wait => Task::none(),
        }
    }

    fn plan_liquidation_fetch_for_instance(
        &self,
        instance: &ChartInstance,
    ) -> LiquidationRequestPlan {
        let coin = self.hyperdash_coin_for_symbol(&instance.symbol);
        liquidation_request_plan(LiquidationPlanContext {
            show_liquidations: instance.show_liquidations,
            liquidation_fetching: instance.liquidation_fetching,
            hyperdash_key_missing: self.hyperdash_api_key.is_empty(),
            symbol: &instance.symbol,
            ticker_muted: self.symbol_key_is_hidden(&instance.symbol),
            coin: coin.as_deref(),
            mark: liquidation_mark_from_ctx(
                instance
                    .asset_ctx
                    .as_ref()
                    .and_then(|ctx| ctx.mark_px.as_deref()),
                instance.chart.candles.last().map(|candle| candle.close),
            ),
        })
    }
}

#[cfg(test)]
mod tests;
