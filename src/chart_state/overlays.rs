use super::{ChartId, ChartInstance};
use crate::app_state::TradingTerminal;
use crate::chart::{OrderOverlay, PositionOverlay};

impl TradingTerminal {
    /// Update position and order overlays for a specific chart.
    pub(crate) fn sync_chart_position_for(&mut self, chart_id: ChartId) {
        let _theme = self.theme();
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.clone(),
            None => return,
        };
        if self.is_ticker_muted(&symbol) {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.chart.active_position = None;
            }
            return;
        }
        let pos_overlay = self
            .account_data
            .as_ref()
            .and_then(|data| {
                data.clearinghouse
                    .asset_positions
                    .iter()
                    .find(|ap| ap.position.coin == symbol)
            })
            .and_then(|ap| {
                let szi: f64 = ap.position.szi.parse().ok()?;
                let entry_px: f64 = ap.position.entry_px.parse().ok()?;
                let liquidation_px = Self::parse_liquidation_px(ap);
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

    pub(crate) fn sync_chart_orders_for(&mut self, chart_id: ChartId) {
        let _theme = self.theme();
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.clone(),
            None => return,
        };
        if self.is_ticker_muted(&symbol) {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.chart.active_orders.clear();
            }
            return;
        }
        let order_overlays = self
            .account_data
            .as_ref()
            .map(|data| {
                data.open_orders
                    .iter()
                    .filter(|o| o.coin == symbol)
                    .filter_map(|o| {
                        let limit_px: f64 = o.limit_px.parse().ok()?;
                        let sz: f64 = o.sz.parse().ok()?;
                        Some(OrderOverlay {
                            coin: o.coin.clone(),
                            limit_px,
                            sz,
                            is_buy: o.side == "B",
                            oid: o.oid,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(inst) = self.charts.get_mut(&chart_id) {
            inst.chart.active_orders = order_overlays;
        }
    }

    pub(crate) fn chart_current_price(inst: &ChartInstance) -> Option<f64> {
        inst.asset_ctx
            .as_ref()
            .and_then(|ctx| ctx.mark_px.as_deref())
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| inst.chart.candles.last().map(|c| c.close))
            .filter(|p| *p > 0.0)
    }

    pub(crate) fn active_mark_price_for_symbol(&self, symbol: &str) -> Option<f64> {
        let _theme = self.theme();
        self.charts
            .values()
            .filter(|inst| inst.symbol == symbol)
            .find_map(Self::chart_current_price)
            .or_else(|| self.resolve_mid_for_symbol(symbol))
    }

    /// Sync overlays for all chart instances.
    pub(crate) fn sync_all_chart_overlays(&mut self) {
        let _theme = self.theme();
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_position_for(id);
            self.sync_chart_orders_for(id);
        }
    }

    /// Sync only order overlays for all chart instances.
    pub(crate) fn sync_all_chart_orders(&mut self) {
        let _theme = self.theme();
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_orders_for(id);
        }
    }
}
