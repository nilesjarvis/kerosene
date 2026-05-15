use super::ChartId;
use crate::app_state::TradingTerminal;
use crate::chart::{OrderOverlay, PositionOverlay, TradeMarker};

use std::collections::{HashMap, HashSet};

impl TradingTerminal {
    /// Update position and order overlays for a specific chart.
    pub(crate) fn sync_chart_position_for(&mut self, chart_id: ChartId) {
        let _theme = self.theme();
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.clone(),
            None => return,
        };
        if self.symbol_key_is_hidden(&symbol) {
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
        if self.symbol_key_is_hidden(&symbol) {
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
                let oid = chase.current_oid?;
                Some(OrderOverlay {
                    coin: chase.coin.clone(),
                    limit_px: chase.current_price,
                    sz: chase.remaining_size,
                    is_buy: chase.is_buy,
                    oid,
                    is_moving: self.pending_move_order_contexts.contains_key(&oid),
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
                            is_moving: self.pending_move_order_contexts.contains_key(&o.oid),
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
        if self.symbol_key_is_hidden(&symbol) {
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

    fn sync_chart_trade_markers_from_index(
        &mut self,
        chart_id: ChartId,
        markers_by_symbol: &HashMap<String, Vec<TradeMarker>>,
    ) {
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.clone(),
            None => return,
        };
        if self.symbol_key_is_hidden(&symbol) {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.chart.trade_markers.clear();
            }
            return;
        }

        if let Some(inst) = self.charts.get_mut(&chart_id) {
            inst.chart.trade_markers = markers_by_symbol.get(&symbol).cloned().unwrap_or_default();
        }
    }

    /// Sync overlays for all chart instances.
    pub(crate) fn sync_all_chart_overlays(&mut self) {
        let _theme = self.theme();
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_position_for(id);
            self.sync_chart_orders_for(id);
        }
        self.sync_all_chart_trade_markers();
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
        let symbols: HashSet<String> = self
            .charts
            .values()
            .map(|inst| inst.symbol.clone())
            .collect();
        self.sync_chart_trade_markers_for_symbols(&symbols);
    }

    /// Sync trade marker overlays only for charts whose symbol changed.
    pub(crate) fn sync_chart_trade_markers_for_symbols(&mut self, symbols: &HashSet<String>) {
        if symbols.is_empty() {
            return;
        }

        let ids: Vec<ChartId> = self
            .charts
            .iter()
            .filter_map(|(id, inst)| symbols.contains(&inst.symbol).then_some(*id))
            .collect();
        if ids.is_empty() {
            return;
        }

        let markers_by_symbol = self
            .account_data
            .as_ref()
            .map(|data| trade_markers_by_symbol(&data.fills, symbols))
            .unwrap_or_default();
        for id in ids {
            self.sync_chart_trade_markers_from_index(id, &markers_by_symbol);
        }
    }
}

fn parse_positive_f64(raw: &str) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn trade_markers_by_symbol(
    fills: &[crate::account::UserFill],
    symbols: &HashSet<String>,
) -> HashMap<String, Vec<TradeMarker>> {
    let mut markers_by_symbol: HashMap<String, Vec<TradeMarker>> = HashMap::new();
    for fill in fills.iter().filter(|fill| symbols.contains(&fill.coin)) {
        let Some(marker) = trade_marker_from_fill(fill) else {
            continue;
        };
        markers_by_symbol
            .entry(fill.coin.clone())
            .or_default()
            .push(marker);
    }
    for markers in markers_by_symbol.values_mut() {
        markers.sort_by_key(|marker| marker.time_ms);
    }
    markers_by_symbol
}

fn trade_markers_for_symbol(fills: &[crate::account::UserFill], symbol: &str) -> Vec<TradeMarker> {
    let mut markers: Vec<_> = fills
        .iter()
        .filter(|fill| fill.coin == symbol)
        .filter_map(trade_marker_from_fill)
        .collect();
    markers.sort_by_key(|marker| marker.time_ms);
    markers
}

fn trade_marker_from_fill(fill: &crate::account::UserFill) -> Option<TradeMarker> {
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
        assert_eq!(markers[0].time_ms, 1);
        assert!(!markers[0].is_buy);
        assert_eq!(markers[1].time_ms, 2);
        assert!(markers[1].is_buy);
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
    fn trade_markers_by_symbol_indexes_only_requested_symbols() {
        let fills = vec![
            fill("BTC", 3, "100", "0.1", "B"),
            fill("ETH", 1, "200", "0.2", "A"),
            fill("BTC", 2, "101", "0.3", "A"),
            fill("SOL", 4, "10", "1", "B"),
        ];
        let symbols = HashSet::from(["BTC".to_string(), "ETH".to_string()]);

        let indexed = trade_markers_by_symbol(&fills, &symbols);

        assert_eq!(indexed.len(), 2);
        assert_eq!(
            indexed
                .get("BTC")
                .expect("BTC markers should be indexed")
                .iter()
                .map(|marker| marker.time_ms)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(
            indexed
                .get("ETH")
                .expect("ETH markers should be indexed")
                .iter()
                .map(|marker| marker.time_ms)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert!(!indexed.contains_key("SOL"));
    }
}
