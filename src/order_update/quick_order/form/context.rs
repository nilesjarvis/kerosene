use super::calculations::parse_positive_finite;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::helpers::positive_finite_value;

// ---------------------------------------------------------------------------
// Quick Order Form Context
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn quick_order_form_parts(&self, id: ChartId) -> Option<(String, bool, f64, bool)> {
        let instance = self.charts.get(&id)?;
        let form = instance.quick_order.as_ref()?;
        Some((
            instance.symbol.clone(),
            form.quantity_is_usd,
            form.price,
            form.is_limit,
        ))
    }

    pub(super) fn quick_order_reference_price(
        &self,
        form_price: f64,
        is_limit: bool,
        symbol: &str,
    ) -> Option<f64> {
        if is_limit {
            positive_finite_value(form_price)
        } else {
            self.resolve_mid_for_symbol(symbol)
                .and_then(positive_finite_value)
        }
    }

    pub(super) fn quick_order_size_decimals(&self, symbol: &str) -> usize {
        self.exchange_symbols
            .iter()
            .find(|exchange_symbol| exchange_symbol.key == symbol)
            .map(|exchange_symbol| exchange_symbol.sz_decimals as usize)
            .unwrap_or(4)
    }

    pub(super) fn quick_order_max_notional(&self, symbol: &str) -> Option<f64> {
        let data = self.account_data.as_ref()?;
        let available_margin = self.visible_available_margin_usdc(data)?;
        let available_margin = positive_finite_value(available_margin)?;

        let max_leverage = data
            .get_leverage_for(symbol, &self.exchange_symbols)
            .map(|(_, leverage, _)| leverage as f64)
            .unwrap_or(1.0);
        positive_finite_value(available_margin * max_leverage)
    }

    pub(super) fn quick_order_percentage_for_quantity(
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

        let Some(target_notional) = positive_finite_value(target_notional) else {
            return 0.0;
        };

        (((target_notional / max_notional) * 100.0) as f32).clamp(0.0, 100.0)
    }
}
