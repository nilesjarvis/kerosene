use crate::app_state::TradingTerminal;
use crate::chart::{OrderOverlay, OrderOverlayPendingState};
use crate::chart_state::ChartId;
use crate::helpers::{parse_positive_finite_number, values_match_approx};
use crate::order_execution::MoveOrderKey;
use crate::order_pending_indicators::PendingOrderIndicatorKind;

// ---------------------------------------------------------------------------
// Chart Order Overlays
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn sync_chart_orders_for(&mut self, chart_id: ChartId) {
        let symbol = match self.charts.get(&chart_id) {
            Some(inst) => inst.symbol.as_str(),
            None => return,
        };
        if self.symbol_key_is_hidden(symbol) {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.chart.active_orders.clear();
                inst.chart.set_pending_market_order_loading([]);
            }
            return;
        }
        let chase_overlays = self
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
                    is_moving: self
                        .pending_move_order_contexts
                        .contains_key(&MoveOrderKey::new(chase.coin.as_str(), oid)),
                    pending_state: None,
                })
            })
            .filter(|order| {
                order.limit_px.is_finite()
                    && order.limit_px > 0.0
                    && order.sz.is_finite()
                    && order.sz > 0.0
            });
        let mut order_overlays: Vec<OrderOverlay> = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data)
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
                            is_moving: self
                                .pending_move_order_contexts
                                .contains_key(&MoveOrderKey::new(o.coin.as_str(), o.oid)),
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
        for (pending_id, pending) in self.pending_order_indicators_for_symbol(symbol) {
            if pending.kind == PendingOrderIndicatorKind::MarketPlacing {
                pending_market_loaders.push((pending_id, pending.is_buy));
                continue;
            }

            let limit_px = parse_positive_finite_number(&pending.price);
            let sz = parse_positive_finite_number(&pending.size);

            let pending_state = Some(match pending.kind {
                PendingOrderIndicatorKind::Placing => OrderOverlayPendingState::Placing,
                PendingOrderIndicatorKind::Cancelling => OrderOverlayPendingState::Cancelling,
                PendingOrderIndicatorKind::Modifying => OrderOverlayPendingState::Modifying,
                PendingOrderIndicatorKind::MarketPlacing => continue,
            });
            if let Some(oid) = pending.oid {
                // Decorating an existing line needs no price/size of its own;
                // TP/SL trigger orders carry sz "0.0" yet must still show a
                // Cancelling state.
                if let Some(existing) = order_overlays.iter_mut().find(|order| order.oid == oid) {
                    if pending.kind == PendingOrderIndicatorKind::Modifying {
                        if let Some(limit_px) = limit_px {
                            existing.limit_px = limit_px;
                        }
                        if let Some(sz) = sz {
                            existing.sz = sz;
                        }
                        existing.is_buy = pending.is_buy;
                    }
                    existing.pending_state = pending_state;
                    existing.is_moving = false;
                }
                // Cancel/modify indicators only decorate the live order line;
                // once the order leaves the authoritative snapshot, drawing a
                // standalone line would resurrect an order that no longer
                // exists.
                continue;
            }

            // A standalone Placing line is drawn from the indicator's own
            // values, so both must be well-formed.
            let (Some(limit_px), Some(sz)) = (limit_px, sz) else {
                continue;
            };

            // The exchange commits orders before the place ack returns, so the
            // websocket can deliver the confirmed order while the Placing
            // indicator is still alive. Suppress the indicator once a matching
            // confirmed line exists, otherwise the trader sees a duplicate.
            let confirmed_order_arrived = order_overlays.iter().any(|order| {
                order.pending_state.is_none()
                    && order.is_buy == pending.is_buy
                    && values_match_approx(order.limit_px, limit_px)
                    && values_match_approx(order.sz, sz)
            });
            if confirmed_order_arrived {
                continue;
            }

            order_overlays.push(OrderOverlay {
                coin: pending.symbol.clone(),
                limit_px,
                sz,
                is_buy: pending.is_buy,
                oid: pending_id,
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
}
