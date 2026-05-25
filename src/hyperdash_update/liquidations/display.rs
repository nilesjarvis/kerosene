use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};

// ---------------------------------------------------------------------------
// Liquidation Display State
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn clear_liquidation_display(instance: &mut ChartInstance) {
        instance.liquidation_data = None;
        reset_liquidation_fetch_state(instance);
        instance.liquidation_status = None;
        instance.chart.liquidation_buckets.clear();
        instance.chart.candle_cache.clear();
    }

    pub(in crate::hyperdash_update::liquidations) fn chart_can_accept_liquidation_result(
        &self,
        chart_id: ChartId,
        coin: &str,
    ) -> bool {
        self.charts.get(&chart_id).is_some_and(|instance| {
            instance.show_liquidations
                && !instance.symbol.is_empty()
                && !self.symbol_key_is_hidden(&instance.symbol)
                && self
                    .hyperdash_coin_for_symbol(&instance.symbol)
                    .is_some_and(|chart_coin| chart_coin == coin)
        })
    }
}

pub(super) fn reset_liquidation_fetch_state(instance: &mut ChartInstance) {
    instance.liquidation_fetching = false;
    instance.liquidation_pending_key = None;
}
