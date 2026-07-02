use crate::account::{AccountAbstractionMode, AccountData};
use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, parse_finite_number, parse_number, positive_finite_value};
use crate::message::Message;
use crate::signing::OrderKind;
use iced::Task;
use std::fmt;

mod order_book;
mod quantity;
pub(in crate::order_update) mod sizing;

#[cfg(test)]
mod tests;

use quantity::toggled_order_quantity_text;
use sizing::{OrderSizingBasis, position_size_for_symbol};

#[derive(Clone, PartialEq)]
pub(crate) struct OrderQuantityProvenance {
    account_address: String,
    account_data_revision: u64,
    symbol_key: String,
    quantity_is_usd: bool,
    order_kind: OrderKind,
    reference_price: Option<f64>,
    reduce_only: bool,
    market_universe: crate::config::MarketUniverseConfig,
}

impl fmt::Debug for OrderQuantityProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderQuantityProvenance")
            .field("account_address", &"<redacted>")
            .field("account_data_revision", &self.account_data_revision)
            .field("symbol_key", &self.symbol_key)
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("order_kind", &self.order_kind)
            .field("reference_price", &self.reference_price)
            .field("reduce_only", &self.reduce_only)
            .field("market_universe", &self.market_universe)
            .finish()
    }
}

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
        self.order_quantity_provenance = None;
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
        if self.order_quantity_provenance.is_some() && self.order_percentage > 0.0 {
            self.update_order_quantity_for_percentage(self.order_percentage);
            return;
        }

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

        let Some((_, data)) = self.connected_order_account_snapshot() else {
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
        let had_percentage_provenance = self.order_quantity_provenance.is_some();
        self.order_quantity_provenance = None;
        let Some((account_address, data)) = self.connected_order_account_snapshot() else {
            if had_percentage_provenance {
                self.order_quantity.clear();
                self.order_percentage = 0.0;
            }
            return;
        };

        let Some(sizing_basis) = self.order_sizing_basis(data) else {
            self.order_quantity = "0".to_string();
            return;
        };

        let reference_price = self.order_reference_price();
        self.order_quantity = sizing_basis.quantity_for_percentage(
            percentage,
            self.order_quantity_is_usd,
            reference_price,
            self.active_symbol_size_decimals(),
        );
        if percentage.is_finite() && percentage > 0.0 {
            self.order_quantity_provenance = Some(OrderQuantityProvenance {
                account_address,
                account_data_revision: self.account_data_revision,
                symbol_key: self.active_symbol.clone(),
                quantity_is_usd: self.order_quantity_is_usd,
                order_kind: self.order_kind,
                reference_price,
                reduce_only: self.order_reduce_only,
                market_universe: self.market_universe.clone(),
            });
        }
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

        if self.is_spot_coin(&self.active_symbol) {
            return self.spot_order_sizing_basis(&self.active_symbol, data);
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
            .filter(|(_, _, is_actual)| *is_actual)
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

    /// Percentage sizing basis for spot markets, which trade wallet balances
    /// rather than USDC margin: when the base token is held, size against the
    /// sellable balance (total - hold); otherwise size buys against the
    /// spendable spot USDC.
    pub(in crate::order_update) fn spot_order_sizing_basis(
        &self,
        symbol: &str,
        data: &AccountData,
    ) -> Option<OrderSizingBasis> {
        if let Some(sellable_size_coin) = self.spot_sellable_base_size(symbol, data) {
            return Some(OrderSizingBasis::SpotSellableBalance { sellable_size_coin });
        }

        spot_spendable_quote_notional(data)
            .map(|max_notional| OrderSizingBasis::MarginNotional { max_notional })
    }

    /// Sellable base-token balance (total - hold) for a spot pair, floored to
    /// the pair's size decimals so a 100% sell never exceeds the balance.
    fn spot_sellable_base_size(&self, symbol: &str, data: &AccountData) -> Option<f64> {
        let exchange_symbol = self
            .exchange_symbols
            .iter()
            .find(|exchange_symbol| exchange_symbol.key == symbol)?;
        let balance = data
            .spot
            .balances
            .iter()
            .find(|balance| balance.coin.eq_ignore_ascii_case(&exchange_symbol.ticker))?;
        let total = parse_finite_number(&balance.total)?;
        let hold = parse_finite_number(&balance.hold)?;
        let sellable = positive_finite_value(total - hold)?;
        floor_to_size_decimals(sellable, exchange_symbol.sz_decimals)
    }

    pub(crate) fn clear_percentage_order_quantity(&mut self) {
        if self.order_quantity_provenance.is_some() {
            self.order_quantity.clear();
            self.order_percentage = 0.0;
        }
        self.order_quantity_provenance = None;
    }

    pub(crate) fn stale_percentage_order_quantity_task(
        &mut self,
        action: &str,
    ) -> Option<Task<Message>> {
        let provenance = self.order_quantity_provenance.clone()?;

        if provenance.symbol_key != self.active_symbol
            || provenance.quantity_is_usd != self.order_quantity_is_usd
            || provenance.reduce_only != self.order_reduce_only
            || provenance.market_universe != self.market_universe
        {
            self.order_status = Some((
                format!("Reselect percentage size before {action}; order context changed"),
                true,
            ));
            return Some(Task::none());
        }

        if !self.order_quantity_is_usd || provenance.reduce_only {
            if provenance.order_kind != self.order_kind {
                self.order_status = Some((
                    format!("Reselect percentage size before {action}; order type changed"),
                    true,
                ));
                return Some(Task::none());
            }

            if !order_reference_price_matches(
                self.order_reference_price(),
                provenance.reference_price,
            ) {
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

        if !data.completeness.positions_actionable {
            self.order_status = Some((
                format!("Positions may be incomplete; refresh account data before {action}"),
                true,
            ));
            return Some(self.refresh_account_data());
        }

        None
    }
}

/// Spot USDC spendable on spot buys. Accounts without balance abstraction can
/// only spend their spot USDC; abstraction modes auto-transfer, so they keep
/// the account-level USDC availability.
fn spot_spendable_quote_notional(data: &AccountData) -> Option<f64> {
    let spendable = if data.account_abstraction == AccountAbstractionMode::Disabled {
        data.spot_available_for_token(0)?
    } else {
        data.available_margin_for_token(0)?
    };
    positive_finite_value(spendable)
}

fn floor_to_size_decimals(size: f64, sz_decimals: u32) -> Option<f64> {
    let decimals = sz_decimals.min(8);
    let factor = 10f64.powi(decimals as i32);
    positive_finite_value(((size * factor) + 1e-9).floor() / factor)
}

fn order_reference_price_matches(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        (None, None) => true,
        _ => false,
    }
}
