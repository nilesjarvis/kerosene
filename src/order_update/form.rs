use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, parse_number, positive_finite_value};
use crate::signing::OrderKind;

mod order_book;
mod quantity;
mod sizing;

#[cfg(test)]
mod tests;

use quantity::toggled_order_quantity_text;
use sizing::{OrderSizingBasis, position_size_for_symbol};

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

        self.refresh_order_percentage_for_current_quantity();
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

        let Some(qty) = parse_number(&self.order_quantity) else {
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
            self.update_order_quantity_for_percentage(value);
            return;
        }

        self.update_order_quantity_for_percentage(value);
    }

    pub(crate) fn handle_set_order_kind(&mut self, kind: OrderKind) {
        self.order_kind = kind;
        let active_symbol = self.active_symbol.clone();
        self.refresh_order_price_for_symbol(&active_symbol);
        self.persist_config();
    }

    pub(crate) fn handle_toggle_reduce_only(&mut self) {
        self.order_reduce_only = !self.order_reduce_only;
        if self.order_percentage > 0.0 {
            self.update_order_quantity_for_percentage(self.order_percentage);
        } else {
            self.refresh_order_percentage_for_current_quantity();
        }
        self.persist_config();
    }

    pub(crate) fn handle_dismiss_order_status(&mut self) {
        self.order_status = None;
    }

    fn order_reference_price(&self) -> Option<f64> {
        if matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc) {
            parse_number(&self.order_price).and_then(positive_finite_value)
        } else {
            self.resolve_mid_for_symbol(&self.active_symbol)
                .and_then(positive_finite_value)
        }
    }

    fn active_symbol_size_decimals(&self) -> usize {
        self.exchange_symbols
            .iter()
            .find(|s| s.key == self.active_symbol)
            .map(|s| s.sz_decimals)
            .unwrap_or(4) as usize
    }

    fn refresh_order_percentage_for_current_quantity(&mut self) {
        let Some(qty) = parse_number(&self.order_quantity) else {
            self.order_percentage = 0.0;
            return;
        };

        let Some(data) = &self.account_data else {
            self.order_percentage = 0.0;
            return;
        };

        let Some(sizing_basis) = self.order_sizing_basis(data) else {
            self.order_percentage = 0.0;
            return;
        };

        self.order_percentage = sizing_basis.percentage_for_quantity(
            qty,
            self.order_quantity_is_usd,
            self.order_reference_price(),
        );
    }

    fn update_order_quantity_for_percentage(&mut self, percentage: f32) {
        let Some(data) = &self.account_data else {
            return;
        };

        let Some(sizing_basis) = self.order_sizing_basis(data) else {
            self.order_quantity = "0".to_string();
            return;
        };

        self.order_quantity = sizing_basis.quantity_for_percentage(
            percentage,
            self.order_quantity_is_usd,
            self.order_reference_price(),
            self.active_symbol_size_decimals(),
        );
    }

    fn order_sizing_basis(&self, data: &AccountData) -> Option<OrderSizingBasis> {
        if self.reduce_only_position_sizing_enabled() {
            return position_size_for_symbol(
                self.visible_clearinghouse_state(data),
                &self.active_symbol,
            )
            .map(|position_size| OrderSizingBasis::ReduceOnlyPosition {
                position_size_coin: position_size,
            });
        }

        let available_margin = if self.is_outcome_coin(&self.active_symbol) {
            data.available_margin_for_token(
                self.outcome_quote_token_index_for_coin(&self.active_symbol),
            )?
        } else {
            self.visible_available_margin_usdc(data)?
        };
        let available_margin = positive_finite_value(available_margin)?;

        let max_leverage = data
            .get_leverage_for(&self.active_symbol, &self.exchange_symbols)
            .map(|(_, leverage, _)| leverage as f64)
            .unwrap_or(1.0);
        positive_finite_value(available_margin * max_leverage)
            .map(|max_notional| OrderSizingBasis::MarginNotional { max_notional })
    }

    fn reduce_only_position_sizing_enabled(&self) -> bool {
        self.order_reduce_only
            && !self.is_spot_coin(&self.active_symbol)
            && !self.is_outcome_coin(&self.active_symbol)
    }
}
