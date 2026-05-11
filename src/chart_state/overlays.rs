use super::ChartId;
use crate::app_state::TradingTerminal;
use crate::chart::{OrderOverlay, PositionOverlay, TradeMarker};

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
        let chase_overlays: Vec<OrderOverlay> = self
            .chase_orders
            .values()
            .filter(|chase| {
                chase.coin == symbol
                    && self.connected_address.as_deref() == Some(chase.account_address.as_str())
            })
            .filter_map(|chase| {
                Some(OrderOverlay {
                    coin: chase.coin.clone(),
                    limit_px: chase.current_price,
                    sz: chase.remaining_size,
                    is_buy: chase.is_buy,
                    oid: chase.current_oid?,
                })
            })
            .filter(|order| {
                order.limit_px.is_finite()
                    && order.limit_px > 0.0
                    && order.sz.is_finite()
                    && order.sz > 0.0
            })
            .collect();
        let mut order_overlays: Vec<OrderOverlay> = self
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
        for chase_order in chase_overlays {
            if let Some(existing) = order_overlays
                .iter_mut()
                .find(|order| order.oid == chase_order.oid)
            {
                *existing = chase_order;
            } else {
                order_overlays.push(chase_order);
            }
        }
        if let Some(inst) = self.charts.get_mut(&chart_id) {
            inst.chart.active_orders = order_overlays;
        }
    }

    pub(crate) fn sync_chart_trade_markers_for(&mut self, chart_id: ChartId) {
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.clone(),
            None => return,
        };
        if self.is_ticker_muted(&symbol) {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.chart.trade_markers.clear();
            }
            return;
        }

        let mut trade_markers = self
            .account_data
            .as_ref()
            .map(|data| trade_markers_for_symbol(&data.fills, &symbol))
            .unwrap_or_default();
        trade_markers.sort_by_key(|marker| marker.time_ms);

        if let Some(inst) = self.charts.get_mut(&chart_id) {
            inst.chart.trade_markers = trade_markers;
        }
    }

    /// Sync overlays for all chart instances.
    pub(crate) fn sync_all_chart_overlays(&mut self) {
        let _theme = self.theme();
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_position_for(id);
            self.sync_chart_orders_for(id);
            self.sync_chart_trade_markers_for(id);
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

    /// Sync only trade marker overlays for all chart instances.
    pub(crate) fn sync_all_chart_trade_markers(&mut self) {
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_trade_markers_for(id);
        }
    }
}

fn parse_positive_f64(raw: &str) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn trade_markers_for_symbol(fills: &[crate::account::UserFill], symbol: &str) -> Vec<TradeMarker> {
    fills
        .iter()
        .filter(|fill| fill.coin == symbol)
        .filter_map(|fill| {
            let price = parse_positive_f64(&fill.px)?;
            let size = parse_positive_f64(&fill.sz)?;
            let is_buy = match fill.side.as_str() {
                "B" => true,
                "A" => false,
                _ => return None,
            };

            Some(TradeMarker {
                time_ms: fill.time,
                price,
                size,
                is_buy,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::UserFill;

    fn fill(coin: &str, time: u64, px: &str, sz: &str, side: &str) -> UserFill {
        UserFill {
            coin: coin.to_string(),
            px: px.to_string(),
            sz: sz.to_string(),
            side: side.to_string(),
            time,
            oid: None,
            dir: "Open Long".to_string(),
            closed_pnl: "0".to_string(),
            fee: "0".to_string(),
        }
    }

    #[test]
    fn trade_markers_for_symbol_maps_valid_fills() {
        let fills = vec![
            fill("BTC", 2, "100", "0.2", "B"),
            fill("BTC", 1, "101", "0.1", "A"),
        ];

        let markers = trade_markers_for_symbol(&fills, "BTC");

        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].time_ms, 2);
        assert!(markers[0].is_buy);
        assert_eq!(markers[1].time_ms, 1);
        assert!(!markers[1].is_buy);
    }

    #[test]
    fn trade_markers_for_symbol_skips_invalid_values_and_sides() {
        let fills = vec![
            fill("BTC", 1, "nan", "0.1", "B"),
            fill("BTC", 2, "100", "0", "B"),
            fill("BTC", 3, "100", "0.1", "X"),
            fill("BTC", 4, "100", "0.1", "A"),
        ];

        let markers = trade_markers_for_symbol(&fills, "BTC");

        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].time_ms, 4);
    }

    #[test]
    fn trade_markers_for_symbol_requires_exact_symbol_match() {
        let fills = vec![
            fill("BTC", 1, "100", "0.1", "B"),
            fill("xyz:BTC", 2, "101", "0.2", "B"),
        ];

        let main_markers = trade_markers_for_symbol(&fills, "BTC");
        let dex_markers = trade_markers_for_symbol(&fills, "xyz:BTC");

        assert_eq!(main_markers.len(), 1);
        assert_eq!(main_markers[0].time_ms, 1);
        assert_eq!(dex_markers.len(), 1);
        assert_eq!(dex_markers[0].time_ms, 2);
    }
}
