mod calculations;
mod context;
mod result;

use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::order_execution::QuickOrderQuantityProvenance;
#[cfg(test)]
use calculations::quick_order_quantity_for_percentage;
use calculations::toggled_quick_order_quantity_text;

impl TradingTerminal {
    pub(crate) fn handle_quick_order_qty_changed(&mut self, id: ChartId, qty: String) {
        let Some((symbol, quantity_is_usd, price, is_limit)) = self.quick_order_form_parts(id)
        else {
            return;
        };

        let qty = if self.is_outcome_coin(&symbol) {
            Self::sanitize_outcome_quantity_input(&qty)
        } else {
            qty
        };
        let percentage = self.quick_order_percentage_for_quantity(
            &symbol,
            quantity_is_usd,
            price,
            is_limit,
            &qty,
        );

        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.quantity = qty;
            form.percentage = percentage;
            form.quantity_provenance = None;
        }
    }

    pub(crate) fn handle_quick_order_percentage_changed(&mut self, id: ChartId, value: f32) {
        let Some((symbol, quantity_is_usd, price, is_limit)) = self.quick_order_form_parts(id)
        else {
            return;
        };

        let percentage = if value.is_finite() {
            value.clamp(0.0, 100.0)
        } else {
            0.0
        };
        let sizing_basis = self.quick_order_sizing_basis(&symbol, quantity_is_usd);
        let reference_price = self.quick_order_reference_price(price, is_limit, &symbol);
        let decimals = self.quick_order_size_decimals(&symbol);
        let quantity = sizing_basis
            .map(|basis| {
                basis.quantity_for_percentage(
                    percentage,
                    quantity_is_usd,
                    reference_price,
                    decimals,
                )
            })
            .unwrap_or_else(|| "0".to_string());
        let quantity_provenance = sizing_basis.and_then(|_| {
            self.quick_order_quantity_provenance(
                &symbol,
                quantity_is_usd,
                percentage,
                reference_price,
                is_limit,
            )
        });

        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.percentage = percentage;
            form.quantity = quantity;
            form.quantity_provenance = quantity_provenance;
        }
    }

    pub(crate) fn handle_quick_order_toggle_denomination(&mut self, id: ChartId) {
        let Some((symbol, quantity_is_usd, price, is_limit)) = self.quick_order_form_parts(id)
        else {
            return;
        };

        let target_is_usd = if self.is_outcome_coin(&symbol) {
            false
        } else {
            !quantity_is_usd
        };
        let reference_price = self.quick_order_reference_price(price, is_limit, &symbol);
        let decimals = self.quick_order_size_decimals(&symbol);

        let (current_quantity, current_percentage, had_provenance) = self
            .charts
            .get(&id)
            .and_then(|instance| instance.quick_order.as_ref())
            .map(|form| {
                (
                    form.quantity.clone(),
                    form.percentage,
                    form.quantity_provenance.is_some(),
                )
            })
            .unwrap_or_else(|| (String::new(), 0.0, false));
        let (quantity, percentage, quantity_provenance) =
            if had_provenance && current_percentage > 0.0 {
                let sizing_basis = self.quick_order_sizing_basis(&symbol, target_is_usd);
                let quantity = sizing_basis
                    .map(|basis| {
                        basis.quantity_for_percentage(
                            current_percentage,
                            target_is_usd,
                            reference_price,
                            decimals,
                        )
                    })
                    .unwrap_or_else(|| "0".to_string());
                let provenance = sizing_basis.and_then(|_| {
                    self.quick_order_quantity_provenance(
                        &symbol,
                        target_is_usd,
                        current_percentage,
                        reference_price,
                        is_limit,
                    )
                });
                (quantity, current_percentage, provenance)
            } else {
                let quantity = toggled_quick_order_quantity_text(
                    &current_quantity,
                    target_is_usd,
                    reference_price,
                    decimals,
                );
                let percentage = self.quick_order_percentage_for_quantity(
                    &symbol,
                    target_is_usd,
                    price,
                    is_limit,
                    &quantity,
                );
                (quantity, percentage, None)
            };

        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.quantity_is_usd = target_is_usd;
            form.quantity = quantity;
            form.percentage = percentage;
            form.quantity_provenance = quantity_provenance;
        }
    }

    pub(crate) fn handle_quick_order_toggle_type(&mut self, id: ChartId) {
        let Some((symbol, quantity_is_usd, price, is_limit)) = self.quick_order_form_parts(id)
        else {
            return;
        };
        let next_is_limit = !is_limit;
        let (current_quantity, current_percentage, had_provenance) = self
            .charts
            .get(&id)
            .and_then(|instance| instance.quick_order.as_ref())
            .map(|form| {
                (
                    form.quantity.clone(),
                    form.percentage,
                    form.quantity_provenance.is_some(),
                )
            })
            .unwrap_or_else(|| (String::new(), 0.0, false));
        let reference_price = self.quick_order_reference_price(price, next_is_limit, &symbol);
        let decimals = self.quick_order_size_decimals(&symbol);
        let (quantity, percentage, quantity_provenance) =
            if had_provenance && current_percentage > 0.0 {
                let sizing_basis = self.quick_order_sizing_basis(&symbol, quantity_is_usd);
                let quantity = sizing_basis
                    .map(|basis| {
                        basis.quantity_for_percentage(
                            current_percentage,
                            quantity_is_usd,
                            reference_price,
                            decimals,
                        )
                    })
                    .unwrap_or_else(|| "0".to_string());
                let provenance = sizing_basis.and_then(|_| {
                    self.quick_order_quantity_provenance(
                        &symbol,
                        quantity_is_usd,
                        current_percentage,
                        reference_price,
                        next_is_limit,
                    )
                });
                (quantity, current_percentage, provenance)
            } else {
                let percentage = self.quick_order_percentage_for_quantity(
                    &symbol,
                    quantity_is_usd,
                    price,
                    next_is_limit,
                    &current_quantity,
                );
                (current_quantity, percentage, None)
            };

        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.is_limit = next_is_limit;
            form.quantity = quantity;
            form.percentage = percentage;
            form.quantity_provenance = quantity_provenance;
            instance.chart.quick_order_limit_price = next_is_limit.then_some(form.price);
            instance.chart.quick_order_line_phase = 0.0;
            instance.last_quick_order_is_limit = form.is_limit;
        }
    }

    pub(crate) fn handle_close_quick_order(&mut self, id: ChartId) {
        if let Some(instance) = self.charts.get_mut(&id) {
            instance.clear_quick_order();
        }
        self.chart_quick_order_surface.remove(&id);
    }

    fn quick_order_quantity_provenance(
        &self,
        symbol: &str,
        quantity_is_usd: bool,
        percentage: f32,
        reference_price: Option<f64>,
        is_limit: bool,
    ) -> Option<QuickOrderQuantityProvenance> {
        if !percentage.is_finite() || percentage <= 0.0 {
            return None;
        }
        let (account_address, _) = self.connected_order_account_snapshot()?;
        Some(QuickOrderQuantityProvenance {
            account_address,
            account_data_revision: self.account_data_revision,
            spot_balances_revision: self.spot_balances_revision,
            symbol_key: symbol.to_string(),
            quantity_is_usd,
            percentage,
            is_limit,
            reference_price,
            reduce_only: self.order_reduce_only,
            market_universe: self.market_universe.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
