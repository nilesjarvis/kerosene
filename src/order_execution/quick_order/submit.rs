use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::config::MarketUniverseConfig;
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OrderSurface, PendingOrderAction, PlaceIntent, PreparedExchangeOrder,
    PriceSource, QuantityDenomination, QuantitySource, QuickOrderForm, QuickOrderRecovery,
    ReduceOnlySource, place_order_task,
};
use crate::signing::ExchangeOrderKind;

use iced::Task;
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuickOrderSubmissionSnapshot {
    pub(crate) surface_id: ChartSurfaceId,
    pub(crate) symbol_key: String,
    pub(crate) form: QuickOrderForm,
    pub(crate) reduce_only: bool,
    pub(crate) market_universe: MarketUniverseConfig,
}

impl TradingTerminal {
    pub(crate) fn quick_order_submission_snapshot(
        &self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
        form: &QuickOrderForm,
    ) -> QuickOrderSubmissionSnapshot {
        QuickOrderSubmissionSnapshot {
            surface_id,
            symbol_key: self
                .charts
                .get(&chart_id)
                .map(|instance| instance.symbol.clone())
                .unwrap_or_default(),
            form: form.clone(),
            reduce_only: self.order_reduce_only,
            market_universe: self.market_universe.clone(),
        }
    }

    fn quick_order_submission_snapshot_matches(
        &self,
        chart_id: ChartId,
        snapshot: &QuickOrderSubmissionSnapshot,
    ) -> bool {
        let Some(instance) = self.charts.get(&chart_id) else {
            return false;
        };
        instance.symbol == snapshot.symbol_key
            && instance.quick_order.as_ref() == Some(&snapshot.form)
            && self.order_reduce_only == snapshot.reduce_only
            && self.market_universe == snapshot.market_universe
            && self.chart_surface_has_quick_order(chart_id, snapshot.surface_id)
    }

    pub(crate) fn handle_submit_quick_order_from_snapshot(
        &mut self,
        chart_id: ChartId,
        is_buy: bool,
        snapshot: QuickOrderSubmissionSnapshot,
    ) -> Task<Message> {
        if !self.quick_order_submission_snapshot_matches(chart_id, &snapshot) {
            self.order_status = Some(("Quick order changed; review and submit again".into(), true));
            return Task::none();
        }

        self.handle_submit_quick_order(chart_id, is_buy)
    }

    pub(crate) fn handle_submit_quick_order(
        &mut self,
        chart_id: ChartId,
        is_buy: bool,
    ) -> Task<Message> {
        if self.reject_if_pending_trading_request("placing a quick order") {
            return Task::none();
        }
        if self.reject_if_account_reconciliation_required("placing a quick order", "account data") {
            return Task::none();
        }

        let _theme = self.theme();
        let Some((key, account_address)) = self.order_signing_context() else {
            return Task::none();
        };

        let quick_order_surface = self.chart_quick_order_surface.remove(&chart_id);
        let form = self
            .charts
            .get_mut(&chart_id)
            .and_then(|inst| inst.take_quick_order());
        let Some(form) = form else {
            return Task::none();
        };

        let chart_symbol = self
            .charts
            .get(&chart_id)
            .map(|inst| inst.symbol.clone())
            .unwrap_or_default();
        if let Some(task) =
            self.stale_quick_order_percentage_task(&form, &chart_symbol, "placing a quick order")
        {
            self.restore_quick_order_form(chart_id, form, quick_order_surface);
            return task;
        }
        let order_kind = if form.is_limit {
            ExchangeOrderKind::Limit
        } else {
            ExchangeOrderKind::Market
        };
        let intent = PlaceIntent {
            surface: OrderSurface::QuickOrder,
            symbol_key: chart_symbol,
            is_buy,
            order_kind,
            price_source: if form.is_limit {
                PriceSource::LimitInput {
                    value: form.price.to_string(),
                    invalid_message: "Invalid price",
                }
            } else {
                PriceSource::MarketWithSlippage {
                    invalid_message: Some("Invalid market price"),
                    usd_size_reference: MarketUsdSizeReference::Mid,
                }
            },
            quantity_source: QuantitySource::UserInput {
                value: form.quantity.clone(),
                denomination: if form.quantity_is_usd {
                    QuantityDenomination::UsdNotional
                } else {
                    QuantityDenomination::Coin
                },
                invalid_message: "Invalid quantity for asset precision",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(self.order_reduce_only),
        };
        let prepared = match self.prepare_place_order(intent) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                self.restore_quick_order_form(chart_id, form, quick_order_surface);
                return Task::none();
            }
        };

        let is_limit = form.is_limit;
        let recovery = QuickOrderRecovery {
            chart_id,
            form,
            surface_id: quick_order_surface,
        };
        self.submit_prepared_quick_order(key, account_address, prepared, is_limit, Some(recovery))
    }

    fn stale_quick_order_percentage_task(
        &mut self,
        form: &QuickOrderForm,
        chart_symbol: &str,
        action: &str,
    ) -> Option<Task<Message>> {
        let provenance = form.quantity_provenance.clone()?;

        if provenance.symbol_key != chart_symbol
            || provenance.quantity_is_usd != form.quantity_is_usd
            || provenance.percentage.to_bits() != form.percentage.to_bits()
            || provenance.is_limit != form.is_limit
            || provenance.reduce_only != self.order_reduce_only
            || provenance.market_universe != self.market_universe
        {
            self.order_status = Some((
                format!("Reselect percentage size before {action}; order context changed"),
                true,
            ));
            return Some(Task::none());
        }

        if !form.quantity_is_usd || provenance.reduce_only {
            let current_reference =
                self.quick_order_reference_price(form.price, form.is_limit, chart_symbol);
            if !quick_order_reference_price_matches(current_reference, provenance.reference_price) {
                self.order_status = Some((
                    format!("Reselect percentage size before {action}; reference price changed"),
                    true,
                ));
                return Some(Task::none());
            }
        }

        if self.account_loading {
            self.order_status = Some((
                format!("Account refresh in progress; wait for fresh account data before {action}"),
                true,
            ));
            return Some(Task::none());
        }
        if self.reject_if_account_reconciliation_required(action, "account data") {
            return Some(Task::none());
        }

        let Some((account_address, data)) = self.connected_order_account_snapshot() else {
            self.order_status = Some((
                format!(
                    "No current account data for percentage size; refresh or reselect size before {action}"
                ),
                true,
            ));
            return Some(self.refresh_account_data());
        };

        if account_address != provenance.account_address {
            self.order_status = Some((
                format!(
                    "Percentage size was calculated for a different account; reselect size before {action}"
                ),
                true,
            ));
            return Some(Task::none());
        }

        if self.account_data_revision != provenance.account_data_revision {
            self.order_status = Some((
                format!(
                    "Percentage size was calculated from an older account snapshot; reselect size before {action}"
                ),
                true,
            ));
            return Some(Task::none());
        }

        if !data.is_fresh_for_position_action(Self::now_ms()) {
            self.order_status = Some((
                format!("Account data is stale for percentage size; refresh before {action}"),
                true,
            ));
            return Some(self.refresh_account_data());
        }

        if !data.completeness.positions_complete {
            self.order_status = Some((
                format!("Positions may be incomplete; refresh account data before {action}"),
                true,
            ));
            return Some(self.refresh_account_data());
        }

        None
    }

    fn submit_prepared_quick_order(
        &mut self,
        key: Zeroizing<String>,
        account_address: String,
        prepared: PreparedExchangeOrder,
        is_limit: bool,
        recovery: Option<QuickOrderRecovery>,
    ) -> Task<Message> {
        let side_str = if prepared.is_buy { "BUY" } else { "SELL" };
        let kind_str = if is_limit { "limit" } else { "market" };
        self.order_status = Some((
            format!(
                "Placing {kind_str} {side_str} {} {}...",
                prepared.size, prepared.symbol_key
            ),
            false,
        ));
        self.pending_order_action = Some(if prepared.is_buy {
            PendingOrderAction::Buy
        } else {
            PendingOrderAction::Sell
        });
        // IOC limit orders are taker orders that never rest, so they project
        // like market orders instead of drawing a provisional resting line.
        let pending_indicator_id = if prepared.order_kind != ExchangeOrderKind::Limit {
            self.add_pending_market_order_placement_indicator(
                account_address.clone(),
                prepared.symbol_key.clone(),
                prepared.is_buy,
                prepared.size.clone(),
                prepared.price.clone(),
            )
        } else {
            self.add_pending_order_placement_indicator(
                account_address.clone(),
                prepared.symbol_key.clone(),
                prepared.is_buy,
                prepared.size.clone(),
                prepared.price.clone(),
            )
        };

        let (request, context) = prepared.place_request_with_context(&account_address);
        place_order_task(key, request, move |result| Message::QuickOrderResult {
            pending_indicator_id,
            context,
            recovery,
            result: Box::new(result),
        })
    }

    pub(crate) fn restore_quick_order_form(
        &mut self,
        chart_id: ChartId,
        form: QuickOrderForm,
        surface_id: Option<ChartSurfaceId>,
    ) {
        let surface_id = surface_id
            .filter(|surface_id| self.quick_order_surface_is_available(chart_id, *surface_id));
        let Some(instance) = self.charts.get_mut(&chart_id) else {
            return;
        };
        instance.set_quick_order(form);
        if let Some(surface_id) = surface_id {
            self.chart_quick_order_surface.insert(chart_id, surface_id);
        }
    }

    pub(crate) fn restore_quick_order_form_if_current(
        &mut self,
        symbol_key: &str,
        recovery: QuickOrderRecovery,
    ) {
        let Some(instance) = self.charts.get(&recovery.chart_id) else {
            return;
        };
        if instance.symbol != symbol_key
            || instance.quick_order.is_some()
            || self
                .chart_quick_order_surface
                .contains_key(&recovery.chart_id)
        {
            return;
        }

        self.restore_quick_order_form(recovery.chart_id, recovery.form, recovery.surface_id);
    }

    fn quick_order_surface_is_available(
        &self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
    ) -> bool {
        let Some(instance) = self.charts.get(&chart_id) else {
            return false;
        };
        if instance.chart.surface_id() != surface_id {
            return false;
        }
        match surface_id {
            ChartSurfaceId::Docked(surface_chart_id) => surface_chart_id == chart_id,
            ChartSurfaceId::Detached(window_id) => self
                .detached_chart_windows
                .get(&window_id)
                .is_some_and(|state| state.chart_id == chart_id),
        }
    }
}

fn quick_order_reference_price_matches(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        (None, None) => true,
        _ => false,
    }
}
