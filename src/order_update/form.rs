use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::signing::OrderKind;

mod quantity;

use quantity::{
    order_percentage_for_quantity, quantity_for_percentage, toggled_order_quantity_text,
};

impl TradingTerminal {
    pub(crate) fn handle_order_price_changed(&mut self, value: String) {
        self.order_price = value;
    }

    pub(crate) fn handle_set_mid_price(&mut self) {
        if let Some(mid) = self.resolve_mid_for_symbol(&self.active_symbol) {
            self.order_price = format_price(mid);
        }
    }

    pub(crate) fn handle_order_quantity_changed(&mut self, value: String) {
        self.order_quantity = if self.is_outcome_coin(&self.active_symbol) {
            Self::sanitize_outcome_quantity_input(&value)
        } else {
            value
        };

        let Ok(qty) = self.order_quantity.parse::<f64>() else {
            self.order_percentage = 0.0;
            return;
        };

        let Some(data) = &self.account_data else {
            self.order_percentage = 0.0;
            return;
        };

        let Some(available_margin) = data.available_margin_usdc() else {
            self.order_percentage = 0.0;
            return;
        };

        let mut max_leverage = 1.0;
        if let Some((_, lev, _)) =
            data.get_leverage_for(&self.active_symbol, &self.exchange_symbols)
        {
            max_leverage = lev as f64;
        }

        let max_notional = available_margin * max_leverage;
        if max_notional <= 0.0 {
            self.order_percentage = 0.0;
            return;
        }

        self.order_percentage = order_percentage_for_quantity(
            qty,
            self.order_quantity_is_usd,
            self.order_reference_price(),
            max_notional,
        );
    }

    pub(crate) fn handle_toggle_order_denomination(&mut self) {
        if self.is_outcome_coin(&self.active_symbol) {
            self.order_quantity_is_usd = false;
            self.order_quantity = Self::sanitize_outcome_quantity_input(&self.order_quantity);
            self.persist_config();
            return;
        }
        self.order_quantity_is_usd = !self.order_quantity_is_usd;
        self.persist_config();

        let Ok(qty) = self.order_quantity.parse::<f64>() else {
            return;
        };

        let Some(parsed_price) = self.order_reference_price() else {
            return;
        };

        let decimals = self.active_symbol_size_decimals();
        if let Some(quantity) =
            toggled_order_quantity_text(qty, self.order_quantity_is_usd, parsed_price, decimals)
        {
            self.order_quantity = quantity;
        }
    }

    pub(crate) fn handle_order_percentage_changed(&mut self, value: f32) {
        self.order_percentage = value;
        if self.is_outcome_coin(&self.active_symbol) {
            self.order_quantity_is_usd = false;
            self.order_quantity = Self::sanitize_outcome_quantity_input(&self.order_quantity);
            return;
        }

        let Some(data) = &self.account_data else {
            return;
        };

        let Some(available_margin) = data.available_margin_usdc() else {
            self.order_quantity = "0".to_string();
            return;
        };

        let mut max_leverage = 1.0;
        if let Some((_, lev, _)) =
            data.get_leverage_for(&self.active_symbol, &self.exchange_symbols)
        {
            max_leverage = lev as f64;
        }

        let max_notional = available_margin * max_leverage;

        self.order_quantity = quantity_for_percentage(
            value,
            max_notional,
            self.order_quantity_is_usd,
            self.order_reference_price(),
            self.active_symbol_size_decimals(),
        );
    }

    pub(crate) fn handle_set_order_kind(&mut self, kind: OrderKind) {
        self.order_kind = kind;
        let active_symbol = self.active_symbol.clone();
        self.refresh_order_price_for_symbol(&active_symbol);
        self.persist_config();
    }

    pub(crate) fn handle_toggle_reduce_only(&mut self) {
        self.order_reduce_only = !self.order_reduce_only;
        self.persist_config();
    }

    pub(crate) fn handle_dismiss_order_status(&mut self) {
        self.order_status = None;
    }

    fn order_reference_price(&self) -> Option<f64> {
        if self.order_kind == OrderKind::Limit || self.order_kind == OrderKind::Chase {
            self.order_price
                .parse::<f64>()
                .ok()
                .filter(|price| price.is_finite() && *price > 0.0)
        } else {
            self.resolve_mid_for_symbol(&self.active_symbol)
                .filter(|price| price.is_finite() && *price > 0.0)
        }
    }

    fn active_symbol_size_decimals(&self) -> usize {
        self.exchange_symbols
            .iter()
            .find(|s| s.key == self.active_symbol)
            .map(|s| s.sz_decimals)
            .unwrap_or(4) as usize
    }
}
