use super::ChartId;
use crate::app_state::TradingTerminal;
use crate::chart::{OrderOverlay, OrderOverlayPendingState, PositionOverlay};
use crate::order_pending_indicators::PendingOrderIndicatorKind;

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

    pub(crate) fn sync_chart_orders_for(&mut self, chart_id: ChartId) {
        let _theme = self.theme();
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.clone(),
            None => return,
        };
        if self.symbol_key_is_hidden(&symbol) {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.chart.active_orders.clear();
                inst.chart.set_pending_market_order_loading([]);
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
                            pending_state: None,
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
        let mut pending_market_loaders = Vec::new();
        for (pending_id, pending) in self.pending_order_indicators_for_symbol(&symbol) {
            if pending.kind == PendingOrderIndicatorKind::MarketPlacing {
                pending_market_loaders.push((pending_id, pending.is_buy));
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
                PendingOrderIndicatorKind::Placing => OrderOverlayPendingState::Placing,
                PendingOrderIndicatorKind::Cancelling => OrderOverlayPendingState::Cancelling,
                PendingOrderIndicatorKind::Modifying => OrderOverlayPendingState::Modifying,
                PendingOrderIndicatorKind::MarketPlacing => continue,
            });
            if let Some(oid) = pending.oid
                && let Some(existing) = order_overlays.iter_mut().find(|order| order.oid == oid)
            {
                if pending.kind == PendingOrderIndicatorKind::Modifying {
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
                oid: pending.oid.unwrap_or(pending_id),
                is_moving: false,
                pending_state,
            });
        }
        if let Some(inst) = self.charts.get_mut(&chart_id) {
            inst.chart.active_orders = order_overlays;
            inst.chart
                .set_pending_market_order_loading(pending_market_loaders);
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

    /// Sync overlays for all chart instances.
    pub(crate) fn sync_all_chart_overlays(&mut self) {
        let _theme = self.theme();
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
        let _theme = self.theme();
        let ids: Vec<ChartId> = self.charts.keys().copied().collect();
        for id in ids {
            self.sync_chart_orders_for(id);
        }
    }

    pub(crate) fn sync_chart_market_reference_prices(&mut self) {
        let references: Vec<_> = self
            .charts
            .iter()
            .map(|(id, instance)| (*id, self.resolve_mid_for_symbol(&instance.symbol)))
            .collect();
        for (id, price) in references {
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.chart.set_market_reference_price(price);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    #[test]
    fn market_reference_prices_sync_from_live_mids() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        terminal.all_mids.insert("BTC".to_string(), 50_000.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());

        terminal.sync_chart_market_reference_prices();

        assert_eq!(
            terminal
                .charts
                .get(&1)
                .unwrap()
                .chart
                .market_reference_price,
            Some(50_000.0)
        );
    }
}
