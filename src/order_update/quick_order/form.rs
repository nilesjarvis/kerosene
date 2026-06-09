mod calculations;
mod context;
mod result;

use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use calculations::{quick_order_quantity_for_percentage, toggled_quick_order_quantity_text};

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
        let max_notional = self.quick_order_max_notional(&symbol).unwrap_or(0.0);
        let reference_price = self.quick_order_reference_price(price, is_limit, &symbol);
        let decimals = self.quick_order_size_decimals(&symbol);
        let quantity = quick_order_quantity_for_percentage(
            percentage,
            max_notional,
            quantity_is_usd,
            reference_price,
            decimals,
        );

        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.percentage = percentage;
            form.quantity = quantity;
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

        let quantity = self
            .charts
            .get(&id)
            .and_then(|instance| instance.quick_order.as_ref())
            .map(|form| {
                toggled_quick_order_quantity_text(
                    &form.quantity,
                    target_is_usd,
                    reference_price,
                    decimals,
                )
            })
            .unwrap_or_default();
        let percentage = self.quick_order_percentage_for_quantity(
            &symbol,
            target_is_usd,
            price,
            is_limit,
            &quantity,
        );

        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.quantity_is_usd = target_is_usd;
            form.quantity = quantity;
            form.percentage = percentage;
        }
    }

    pub(crate) fn handle_quick_order_toggle_type(&mut self, id: ChartId) {
        let Some((symbol, quantity_is_usd, price, is_limit)) = self.quick_order_form_parts(id)
        else {
            return;
        };
        let next_is_limit = !is_limit;
        let quantity = self
            .charts
            .get(&id)
            .and_then(|instance| instance.quick_order.as_ref())
            .map(|form| form.quantity.clone())
            .unwrap_or_default();
        let percentage = self.quick_order_percentage_for_quantity(
            &symbol,
            quantity_is_usd,
            price,
            next_is_limit,
            &quantity,
        );

        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.is_limit = next_is_limit;
            form.percentage = percentage;
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
}

#[cfg(test)]
mod tests;
