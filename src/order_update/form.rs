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
    spot_balances_revision: u64,
    symbol_key: String,
    quantity_is_usd: bool,
    percentage: f32,
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
            .field("spot_balances_revision", &self.spot_balances_revision)
            .field("symbol_key", &self.symbol_key)
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("percentage", &"<redacted>")
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
        self.exchange_symbol_for_key(&self.active_symbol)
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
                spot_balances_revision: self.spot_balances_revision,
                symbol_key: self.active_symbol.clone(),
                quantity_is_usd: self.order_quantity_is_usd,
                percentage,
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

        if self
            .exchange_symbol_for_key(&self.active_symbol)
            .is_some_and(|symbol| symbol.market_type == crate::api::MarketType::Spot)
        {
            if !self.spot_usd_denomination_supported(&self.active_symbol) {
                return None;
            }
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
        if self
            .exchange_symbol_for_key(symbol)
            .is_none_or(|exchange_symbol| {
                exchange_symbol.market_type != crate::api::MarketType::Spot
            })
        {
            return None;
        }
        if let Some(sellable_size_coin) = self.spot_sellable_base_size(symbol, data) {
            return Some(OrderSizingBasis::SpotSellableBalance { sellable_size_coin });
        }

        self.spot_spendable_quote_balance(symbol, data)
            .and_then(positive_finite_value)
            .map(|max_notional| OrderSizingBasis::MarginNotional { max_notional })
    }

    /// Exact sizing basis for a spot action. The shared ticket has two action
    /// buttons, so its preview cannot know a side; submission must recompute
    /// the percentage-derived quantity with the side that was actually
    /// clicked.
    pub(in crate::order_update) fn spot_order_sizing_basis_for_side(
        &self,
        symbol: &str,
        data: &AccountData,
        is_buy: bool,
    ) -> Option<OrderSizingBasis> {
        if self
            .exchange_symbol_for_key(symbol)
            .is_none_or(|exchange_symbol| {
                exchange_symbol.market_type != crate::api::MarketType::Spot
            })
        {
            return None;
        }
        if !is_buy {
            return self
                .spot_sellable_base_size(symbol, data)
                .map(|sellable_size_coin| OrderSizingBasis::SpotSellableBalance {
                    sellable_size_coin,
                });
        }

        self.spot_spendable_quote_balance(symbol, data)
            .and_then(positive_finite_value)
            .map(|max_notional| OrderSizingBasis::MarginNotional { max_notional })
    }

    pub(crate) fn spot_percentage_quantity_for_side(
        &self,
        symbol: &str,
        data: &AccountData,
        is_buy: bool,
        percentage: f32,
        quantity_is_usd: bool,
        reference_price: Option<f64>,
    ) -> Option<String> {
        if !self.spot_usd_denomination_supported(symbol) {
            return None;
        }
        let decimals = self.exchange_symbol_for_key(symbol)?.sz_decimals as usize;
        self.spot_order_sizing_basis_for_side(symbol, data, is_buy)
            .map(|basis| {
                basis.quantity_for_percentage(
                    percentage,
                    quantity_is_usd,
                    reference_price,
                    decimals,
                )
            })
    }

    pub(crate) fn spot_percentage_available_balance_for_side(
        &self,
        symbol: &str,
        data: &AccountData,
        is_buy: bool,
    ) -> Option<f64> {
        match self.spot_order_sizing_basis_for_side(symbol, data, is_buy)? {
            OrderSizingBasis::MarginNotional { max_notional } => Some(max_notional),
            OrderSizingBasis::SpotSellableBalance { sellable_size_coin } => {
                Some(sellable_size_coin)
            }
            OrderSizingBasis::ReduceOnlyPosition { .. } => None,
        }
    }

    pub(crate) fn ticket_spot_percentage_balance_for_side(
        &self,
        is_buy: bool,
    ) -> Option<(f64, f32)> {
        let provenance = self.order_quantity_provenance.as_ref()?;
        if provenance.symbol_key != self.active_symbol
            || !provenance.percentage.is_finite()
            || provenance.percentage <= 0.0
            || provenance.percentage > 100.0
            || self
                .exchange_symbol_for_key(&self.active_symbol)
                .is_none_or(|symbol| symbol.market_type != crate::api::MarketType::Spot)
        {
            return None;
        }
        let data = self.account_data_for_order_account(&provenance.account_address)?;
        self.spot_percentage_available_balance_for_side(&self.active_symbol, data, is_buy)
            .map(|balance| (balance, provenance.percentage))
    }

    pub(crate) fn spot_usd_denomination_supported(&self, symbol: &str) -> bool {
        self.exchange_symbol_for_key(symbol)
            .filter(|exchange_symbol| exchange_symbol.market_type == crate::api::MarketType::Spot)
            .is_some_and(crate::api::ExchangeSymbol::spot_quote_is_usd_stable)
    }

    pub(crate) fn validate_spot_quantity_denomination(
        &self,
        symbol: &str,
        _quantity_is_usd: bool,
    ) -> Result<(), String> {
        let Some(exchange_symbol) = self.exchange_symbol_for_key(symbol) else {
            return Ok(());
        };
        if exchange_symbol.market_type != crate::api::MarketType::Spot {
            return Ok(());
        }
        if self.spot_usd_denomination_supported(symbol) {
            return Ok(());
        }

        Err(format!(
            "Spot trading is unavailable for {} because quote-token USD valuation and accounting are not verified",
            self.display_name_for_symbol(symbol)
        ))
    }

    pub(crate) fn validate_spot_automation_quote(
        &self,
        symbol: &str,
        automation: &str,
    ) -> Result<(), String> {
        let Some(exchange_symbol) = self.exchange_symbol_for_key(symbol) else {
            return Ok(());
        };
        if exchange_symbol.market_type != crate::api::MarketType::Spot
            || exchange_symbol.spot_quote_is_usd_stable()
        {
            return Ok(());
        }

        Err(format!(
            "{automation} is unavailable for {} because its quote token cannot be converted safely to USD",
            self.display_name_for_symbol(symbol)
        ))
    }

    pub(crate) fn spot_quote_token_index(&self, symbol: &str) -> Option<u32> {
        let exchange_symbol = self.exchange_symbol_for_key(symbol)?;
        if exchange_symbol.market_type != crate::api::MarketType::Spot {
            return None;
        }

        // `collateral_token` carries the quote-token index for spot symbols.
        // Legacy cached USDC pairs predate that metadata; only that unambiguous
        // quote is safe to recover. Never guess token 0 for another quote.
        exchange_symbol.collateral_token.or_else(|| {
            exchange_symbol
                .display_name
                .as_deref()
                .and_then(|display| display.rsplit_once('/'))
                .and_then(|(_, quote)| quote.eq_ignore_ascii_case("USDC").then_some(0))
        })
    }

    pub(crate) fn spot_spendable_quote_balance(
        &self,
        symbol: &str,
        data: &AccountData,
    ) -> Option<f64> {
        let quote_token = self.spot_quote_token_index(symbol)?;
        let spendable = if data.account_abstraction == AccountAbstractionMode::Disabled {
            data.spot_available_for_token(quote_token)?
        } else {
            data.available_margin_for_token(quote_token)?
        };
        spendable.is_finite().then_some(spendable)
    }

    /// Sellable base-token balance (total - hold) for a spot pair, floored to
    /// the pair's size decimals so a 100% sell never exceeds the balance.
    fn spot_sellable_base_size(&self, symbol: &str, data: &AccountData) -> Option<f64> {
        let exchange_symbol = self.exchange_symbol_for_key(symbol)?;
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
        is_buy: bool,
    ) -> Option<Task<Message>> {
        let provenance = self.order_quantity_provenance.clone()?;

        if provenance.symbol_key != self.active_symbol
            || provenance.quantity_is_usd != self.order_quantity_is_usd
            || provenance.percentage.to_bits() != self.order_percentage.to_bits()
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

        let is_spot = self
            .exchange_symbol_for_key(&self.active_symbol)
            .is_some_and(|symbol| symbol.market_type == crate::api::MarketType::Spot);
        if is_spot && self.spot_balances_revision != provenance.spot_balances_revision {
            self.order_status = Some((
                format!(
                    "Percentage size was calculated from older spot balances; reselect size before {action}"
                ),
                true,
            ));
            return Some(Task::none());
        }

        if !is_spot && self.account_data_revision != provenance.account_data_revision {
            self.order_status = Some((
                format!(
                    "Percentage size was calculated from an older account snapshot; reselect size before {action}"
                ),
                true,
            ));
            return Some(Task::none());
        }

        if is_spot {
            if let Err(message) = self.validate_spot_quantity_denomination(
                &self.active_symbol,
                self.order_quantity_is_usd,
            ) {
                self.order_status = Some((message, true));
                return Some(Task::none());
            }
            if !data.completeness.spot_balances_complete {
                self.order_status = Some((
                    format!(
                        "Spot balances may be incomplete; refresh account data before {action}"
                    ),
                    true,
                ));
                return Some(self.refresh_account_data());
            }
            if !data.is_fresh_for_spot_balance_action(Self::now_ms()) {
                self.order_status = Some((
                    format!("Spot balances are stale for percentage size; refresh before {action}"),
                    true,
                ));
                return Some(self.refresh_account_data());
            }

            let quantity = self.spot_percentage_quantity_for_side(
                &self.active_symbol,
                data,
                is_buy,
                provenance.percentage,
                self.order_quantity_is_usd,
                self.order_reference_price(),
            );
            let Some(quantity) = quantity else {
                let side = if is_buy { "buying" } else { "selling" };
                self.order_status = Some((
                    format!(
                        "No verified spot balance available for {side}; refresh metadata and account data before {action}"
                    ),
                    true,
                ));
                return Some(Task::none());
            };
            if self.order_quantity != quantity {
                self.order_quantity = quantity;
                let side = if is_buy { "Buy" } else { "Sell" };
                self.order_status = Some((
                    format!(
                        "{side} percentage size was recalculated for the selected side; review the quantity and submit again"
                    ),
                    true,
                ));
                return Some(Task::none());
            }
            return None;
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
