use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::hyperdash_api::{LiquidationLevel, bucket_liquidations, fetch_liquidation_levels};
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(in crate::hyperdash_update) fn toggle_liquidation_overlay(
        &mut self,
        chart_id: ChartId,
    ) -> Task<Message> {
        let coin = self
            .charts
            .get(&chart_id)
            .filter(|inst| !self.is_ticker_muted(&inst.symbol))
            .and_then(|inst| self.hyperdash_coin_for_symbol(&inst.symbol));
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.show_liquidations = !instance.show_liquidations;
            if !instance.show_liquidations {
                instance.chart.liquidation_buckets.clear();
                instance.chart.candle_cache.clear();
            }
            if instance.show_liquidations
                && instance.liquidation_data.is_none()
                && !self.hyperdash_api_key.is_empty()
                && !instance.symbol.is_empty()
            {
                let mark = liquidation_mark_from_ctx(
                    instance
                        .asset_ctx
                        .as_ref()
                        .and_then(|ctx| ctx.mark_px.as_deref()),
                );
                if let Some(mark) = mark {
                    let Some(coin) = coin else {
                        self.push_toast(
                            "Liquidation overlay is only available for perp symbols".to_string(),
                            true,
                        );
                        return Task::none();
                    };
                    return liquidation_fetch_task(
                        chart_id,
                        coin,
                        mark,
                        self.hyperdash_api_key.trim().to_string(),
                    );
                }
            }
        }

        Task::none()
    }

    pub(in crate::hyperdash_update) fn apply_chart_liquidation_loaded(
        &mut self,
        chart_id: ChartId,
        result: Result<LiquidationLevel, String>,
    ) -> Task<Message> {
        if self
            .charts
            .get(&chart_id)
            .is_some_and(|instance| self.is_ticker_muted(&instance.symbol))
        {
            return Task::none();
        }
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            match result {
                Ok(data) => {
                    let buckets = bucket_liquidations(&data.liquidations, data.min, data.max, 200);
                    instance.chart.liquidation_buckets = buckets;
                    instance.liquidation_data = Some(data);
                    instance.chart.candle_cache.clear();
                }
                Err(_e) => {
                    instance.liquidation_data = None;
                    instance.chart.liquidation_buckets.clear();
                    instance.chart.candle_cache.clear();
                }
            }
        }

        Task::none()
    }

    pub(in crate::hyperdash_update) fn refresh_liquidations(&self) -> Task<Message> {
        if self.hyperdash_api_key.is_empty() {
            return Task::none();
        }
        let mut tasks = Vec::new();
        for instance in self.charts.values() {
            if instance.show_liquidations
                && !instance.symbol.is_empty()
                && !self.is_ticker_muted(&instance.symbol)
            {
                let Some(coin) = self.hyperdash_coin_for_symbol(&instance.symbol) else {
                    continue;
                };
                if let Some(mark) = liquidation_mark_from_ctx(
                    instance
                        .asset_ctx
                        .as_ref()
                        .and_then(|ctx| ctx.mark_px.as_deref()),
                ) {
                    tasks.push(liquidation_fetch_task(
                        instance.id,
                        coin,
                        mark,
                        self.hyperdash_api_key.trim().to_string(),
                    ));
                }
            }
        }
        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }
}

fn liquidation_mark_from_ctx(mark_px: Option<&str>) -> Option<f64> {
    let mark = mark_px?.trim().parse::<f64>().ok()?;
    (mark.is_finite() && mark > 0.0).then_some(mark)
}

fn liquidation_fetch_task(id: ChartId, coin: String, mark: f64, api_key: String) -> Task<Message> {
    Task::perform(
        fetch_liquidation_levels(coin, 0.0, mark * 2.0, api_key),
        move |r| Message::ChartLiquidationLoaded(id, Box::new(r)),
    )
}

#[cfg(test)]
mod tests {
    use super::liquidation_mark_from_ctx;

    #[test]
    fn liquidation_mark_parser_rejects_missing_nonpositive_or_nonfinite_values() {
        assert_eq!(liquidation_mark_from_ctx(Some("100.5")), Some(100.5));
        assert_eq!(liquidation_mark_from_ctx(None), None);
        assert_eq!(liquidation_mark_from_ctx(Some("0")), None);
        assert_eq!(liquidation_mark_from_ctx(Some("-1")), None);
        assert_eq!(liquidation_mark_from_ctx(Some("NaN")), None);
        assert_eq!(liquidation_mark_from_ctx(Some("bad")), None);
    }
}
