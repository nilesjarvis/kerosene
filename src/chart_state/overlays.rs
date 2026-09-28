use super::ChartId;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::chart::PositionOverlay;
use crate::helpers::positive_finite_value;

mod orders;
mod trades;

use self::trades::trade_markers_for_symbol;

impl TradingTerminal {
    /// Update the position overlay for a specific chart.
    pub(crate) fn sync_chart_position_for(&mut self, chart_id: ChartId) {
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.as_str(),
            None => return,
        };
        if self.symbol_key_is_hidden(symbol) {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.chart.active_position = None;
            }
            return;
        }
        let pos_overlay = self
            .account_positions_with_outcomes()
            .into_iter()
            .find(|ap| ap.position.coin == symbol)
            .and_then(|ap| {
                let szi: f64 = ap.position.szi.parse().ok()?;
                let entry_px: f64 = ap.position.entry_px.parse().ok()?;
                let liquidation_px = Self::parse_liquidation_px(&ap);
                if szi.abs() < 1e-12 {
                    return None;
                }
                Some(PositionOverlay {
                    entry_px,
                    szi,
                    liquidation_px,
                })
            });
        if let Some(inst) = self.charts.get_mut(&chart_id) {
            inst.chart.active_position = pos_overlay;
        }
    }

    pub(crate) fn sync_chart_trade_markers_for(&mut self, chart_id: ChartId) {
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.as_str(),
            None => return,
        };
        if self.symbol_key_is_hidden(symbol) {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.chart.trade_markers.clear();
            }
            return;
        }

        let mut trade_markers = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data)
            .map(|data| trade_markers_for_symbol(&data.fills, symbol))
            .unwrap_or_default();
        trade_markers.sort_by_key(|marker| marker.time_ms);

        if let Some(inst) = self.charts.get_mut(&chart_id) {
            inst.chart.trade_markers = trade_markers;
        }
    }

    /// Sync overlays for all chart instances.
    pub(crate) fn sync_all_chart_overlays(&mut self) {
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_position_for(id);
            self.sync_chart_orders_for(id);
            self.sync_chart_trade_markers_for(id);
        }
        self.sync_chart_market_reference_prices();
    }

    /// Sync only order overlays for all chart instances.
    pub(crate) fn sync_all_chart_orders(&mut self) {
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_orders_for(id);
        }
    }

    pub(crate) fn sync_chart_market_reference_prices(&mut self) {
        let references: Vec<_> = self
            .charts
            .iter()
            .map(|(id, instance)| {
                (
                    *id,
                    self.resolve_mid_for_symbol(&instance.symbol),
                    self.chart_hud_max_notional_for_symbol(&instance.symbol),
                )
            })
            .collect();
        for (id, price, max_notional) in references {
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.chart.set_market_reference_price(price);
                instance.chart.set_hud_max_notional(max_notional);
            }
        }
    }

    pub(crate) fn chart_hud_max_notional_for_symbol(&self, symbol: &str) -> Option<f64> {
        let exchange_symbol = self
            .exchange_symbols
            .iter()
            .find(|exchange_symbol| exchange_symbol.key == symbol)?;
        if exchange_symbol.market_type == MarketType::Outcome
            || !self.exchange_symbol_is_orderable(exchange_symbol)
        {
            return None;
        }

        let (_, data) = self.connected_order_account_snapshot()?;
        let available_margin = positive_finite_value(self.visible_available_margin_usdc(data)?)?;
        let leverage = data
            .get_leverage_for(symbol, &self.exchange_symbols)
            .filter(|(_, _, is_actual)| *is_actual)
            .map(|(_, leverage, _)| leverage as f64)
            .unwrap_or(1.0);

        positive_finite_value(available_margin * leverage)
    }

    /// Sync only trade marker overlays for all chart instances.
    pub(crate) fn sync_all_chart_trade_markers(&mut self) {
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_trade_markers_for(id);
        }
    }
}

#[cfg(test)]
mod tests;
