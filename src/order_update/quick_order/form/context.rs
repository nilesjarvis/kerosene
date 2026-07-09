use super::calculations::parse_positive_finite;
use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::helpers::positive_finite_value;
use crate::order_update::form::sizing::{OrderSizingBasis, position_size_for_symbol};

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

    pub(crate) fn quick_order_reference_price(
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

    #[cfg(test)]
    pub(super) fn quick_order_max_notional(&self, symbol: &str) -> Option<f64> {
        let (_, data) = self.connected_order_account_snapshot()?;
        self.quick_order_margin_notional(symbol, data)
    }

    pub(super) fn quick_order_sizing_basis(
        &self,
        symbol: &str,
        _quantity_is_usd: bool,
    ) -> Option<OrderSizingBasis> {
        let (_, data) = self.connected_order_account_snapshot()?;
        if self.quick_order_reduce_only_position_sizing_enabled(symbol) {
            return position_size_for_symbol(self.visible_clearinghouse_state(data), symbol).map(
                |position_size| OrderSizingBasis::ReduceOnlyPosition {
                    position_size_coin: position_size,
                },
            );
        }

        if self
            .exchange_symbol_for_key(symbol)
            .is_some_and(|exchange_symbol| {
                exchange_symbol.market_type == crate::api::MarketType::Spot
            })
        {
            if !self.spot_usd_denomination_supported(symbol) {
                return None;
            }
            return self.spot_order_sizing_basis(symbol, data);
        }

        self.quick_order_margin_notional(symbol, data)
            .map(|max_notional| OrderSizingBasis::MarginNotional { max_notional })
    }

    fn quick_order_margin_notional(&self, symbol: &str, data: &AccountData) -> Option<f64> {
        let available_margin = self.visible_available_margin_usdc(data)?;
        let available_margin = positive_finite_value(available_margin)?;

        let max_leverage = data
            .get_leverage_for(symbol, &self.exchange_symbols)
            .filter(|(_, _, is_actual)| *is_actual)
            .map(|(_, leverage, _)| leverage as f64)
            .unwrap_or(1.0);
        positive_finite_value(available_margin * max_leverage)
    }

    fn quick_order_reduce_only_position_sizing_enabled(&self, symbol: &str) -> bool {
        self.order_reduce_only && !self.is_spot_coin(symbol) && !self.is_outcome_coin(symbol)
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
        let Some(sizing_basis) = self.quick_order_sizing_basis(symbol, quantity_is_usd) else {
            return 0.0;
        };
        sizing_basis.percentage_for_quantity(
            quantity,
            quantity_is_usd,
            self.quick_order_reference_price(form_price, is_limit, symbol),
        )
    }
}
