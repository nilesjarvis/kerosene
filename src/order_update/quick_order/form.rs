use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::signing::ExchangeResponse;
use iced::Task;

use super::super::results::result_requires_account_refresh;

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

        self.order_quantity_is_usd = target_is_usd;
        self.persist_config();
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
    }

    pub(crate) fn handle_quick_order_result(
        &mut self,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        match result {
            Ok(resp) => {
                let is_err = resp.is_error();
                self.set_order_status(resp.summary(), is_err);
            }
            Err(e) => {
                self.set_order_status(e, true);
            }
        }
        if should_refresh {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }

    fn quick_order_form_parts(&self, id: ChartId) -> Option<(String, bool, f64, bool)> {
        let instance = self.charts.get(&id)?;
        let form = instance.quick_order.as_ref()?;
        Some((
            instance.symbol.clone(),
            form.quantity_is_usd,
            form.price,
            form.is_limit,
        ))
    }

    fn quick_order_reference_price(
        &self,
        form_price: f64,
        is_limit: bool,
        symbol: &str,
    ) -> Option<f64> {
        if is_limit {
            (form_price.is_finite() && form_price > 0.0).then_some(form_price)
        } else {
            self.resolve_mid_for_symbol(symbol)
                .filter(|price| price.is_finite() && *price > 0.0)
        }
    }

    fn quick_order_size_decimals(&self, symbol: &str) -> usize {
        self.exchange_symbols
            .iter()
            .find(|exchange_symbol| exchange_symbol.key == symbol)
            .map(|exchange_symbol| exchange_symbol.sz_decimals as usize)
            .unwrap_or(4)
    }

    fn quick_order_max_notional(&self, symbol: &str) -> Option<f64> {
        let data = self.account_data.as_ref()?;
        let available_margin = data.available_margin_usdc()?;
        if !available_margin.is_finite() || available_margin <= 0.0 {
            return None;
        }

        let max_leverage = data
            .get_leverage_for(symbol, &self.exchange_symbols)
            .map(|(_, leverage, _)| leverage as f64)
            .unwrap_or(1.0);
        let max_notional = available_margin * max_leverage;
        (max_notional.is_finite() && max_notional > 0.0).then_some(max_notional)
    }

    fn quick_order_percentage_for_quantity(
        &self,
        symbol: &str,
        quantity_is_usd: bool,
        form_price: f64,
        is_limit: bool,
        quantity: &str,
    ) -> f32 {
        let Some(quantity) = parse_positive_finite(quantity) else {
            return 0.0;
        };
        let Some(max_notional) = self.quick_order_max_notional(symbol) else {
            return 0.0;
        };

        let target_notional = if quantity_is_usd {
            quantity
        } else {
            let Some(reference_price) =
                self.quick_order_reference_price(form_price, is_limit, symbol)
            else {
                return 0.0;
            };
            quantity * reference_price
        };

        if !target_notional.is_finite() {
            return 0.0;
        }

        (((target_notional / max_notional) * 100.0) as f32).clamp(0.0, 100.0)
    }
}

fn parse_positive_finite(value: &str) -> Option<f64> {
    let parsed = value.trim().parse::<f64>().ok()?;
    (parsed.is_finite() && parsed > 0.0).then_some(parsed)
}

fn quick_order_quantity_for_percentage(
    percentage: f32,
    max_notional: f64,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
    decimals: usize,
) -> String {
    if !percentage.is_finite() || !max_notional.is_finite() || max_notional <= 0.0 {
        return "0".to_string();
    }

    let target_notional = max_notional * (percentage.clamp(0.0, 100.0) as f64 / 100.0);
    if quantity_is_usd {
        return format!("{target_notional:.2}");
    }

    if let Some(reference_price) = reference_price.filter(|price| price.is_finite() && *price > 0.0)
    {
        let target_coin = target_notional / reference_price;
        format!("{target_coin:.decimals$}")
    } else {
        "0".to_string()
    }
}

fn toggled_quick_order_quantity_text(
    quantity: &str,
    target_is_usd: bool,
    reference_price: Option<f64>,
    decimals: usize,
) -> String {
    let Some(quantity) = parse_positive_finite(quantity) else {
        return quantity.to_string();
    };
    let Some(reference_price) = reference_price.filter(|price| price.is_finite() && *price > 0.0)
    else {
        return quantity.to_string();
    };

    if target_is_usd {
        format!("{:.2}", quantity * reference_price)
    } else {
        format!("{:.decimals$}", quantity / reference_price)
    }
}

#[cfg(test)]
mod tests {
    use super::{quick_order_quantity_for_percentage, toggled_quick_order_quantity_text};

    #[test]
    fn quick_order_percentage_quantity_formats_usd_and_coin() {
        assert_eq!(
            quick_order_quantity_for_percentage(25.0, 1_000.0, true, Some(100.0), 4),
            "250.00"
        );
        assert_eq!(
            quick_order_quantity_for_percentage(25.0, 1_000.0, false, Some(100.0), 4),
            "2.5000"
        );
    }

    #[test]
    fn quick_order_percentage_quantity_rejects_invalid_inputs() {
        assert_eq!(
            quick_order_quantity_for_percentage(f32::NAN, 1_000.0, true, Some(100.0), 4),
            "0"
        );
        assert_eq!(
            quick_order_quantity_for_percentage(25.0, 0.0, true, Some(100.0), 4),
            "0"
        );
        assert_eq!(
            quick_order_quantity_for_percentage(25.0, 1_000.0, false, None, 4),
            "0"
        );
    }

    #[test]
    fn toggled_quick_order_quantity_converts_when_reference_price_is_available() {
        assert_eq!(
            toggled_quick_order_quantity_text("2.5", true, Some(100.0), 4),
            "250.00"
        );
        assert_eq!(
            toggled_quick_order_quantity_text("250", false, Some(100.0), 4),
            "2.5000"
        );
    }
}
