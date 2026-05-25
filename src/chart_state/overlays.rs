use super::ChartId;
use crate::app_state::TradingTerminal;
use crate::chart::{OrderOverlay, OrderOverlayPendingState, PositionOverlay};
use crate::optimistic_updates::PendingOrderChangeKind;

mod trades;

use self::trades::trade_markers_for_symbol;

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
            .projected_positions()
            .into_iter()
            .find(|row| row.asset_position.position.coin == symbol)
            .and_then(|row| {
                let ap = row.asset_position;
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
                    pending_state: None,
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
            .projected_open_orders()
            .into_iter()
            .filter(|row| row.order.coin == symbol)
            .filter_map(|row| {
                let limit_px: f64 = row.order.limit_px.parse().ok()?;
                let sz: f64 = row.order.sz.parse().ok()?;
                Some(OrderOverlay {
                    coin: row.order.coin.clone(),
                    limit_px,
                    sz,
                    is_buy: row.order.side == "B",
                    oid: row.order.oid,
                    is_moving: self
                        .pending_move_order_contexts
                        .contains_key(&row.order.oid),
                    pending_state: None,
                })
            })
            .collect();
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
        for pending in self.projected_pending_order_changes() {
            if pending.symbol != symbol {
                continue;
            }
            let Some(limit_px) = pending.price.parse::<f64>().ok() else {
                continue;
            };
            let Some(sz) = pending.size.parse::<f64>().ok() else {
                continue;
            };
            if !limit_px.is_finite() || limit_px <= 0.0 || !sz.is_finite() || sz <= 0.0 {
                continue;
            }

            let pending_state = Some(match pending.kind {
                PendingOrderChangeKind::Placing => OrderOverlayPendingState::Placing,
                PendingOrderChangeKind::Cancelling => OrderOverlayPendingState::Cancelling,
                PendingOrderChangeKind::Modifying => OrderOverlayPendingState::Modifying,
            });
            if let Some(oid) = pending.oid
                && let Some(existing) = order_overlays.iter_mut().find(|order| order.oid == oid)
            {
                if pending.kind == PendingOrderChangeKind::Modifying {
                    existing.limit_px = limit_px;
                    existing.sz = sz;
                    existing.is_buy = pending.is_buy;
                }
                existing.pending_state = pending_state;
                existing.is_moving = false;
                continue;
            }

            order_overlays.push(OrderOverlay {
                coin: pending.symbol,
                limit_px,
                sz,
                is_buy: pending.is_buy,
                oid: pending.oid.unwrap_or(pending.pending_id),
                is_moving: false,
                pending_state,
            });
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

        let fills = self.merged_user_fills();
        let mut trade_markers = trade_markers_for_symbol(&fills, &symbol);
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
