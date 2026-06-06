use super::PendingOrderAction;
use super::pricing::{rounded_market_price, slipped_market_price};
use super::sizing::order_size_from_quantity_input;
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{OrderKind, float_to_wire, place_order, round_price};

use iced::Task;

mod inputs;
#[cfg(test)]
mod tests;

use inputs::parse_positive_amount;

#[derive(Debug, Clone, PartialEq)]
struct PreparedOrderSubmission {
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    order_kind: OrderKind,
    reduce_only: bool,
    is_outcome: bool,
}

impl TradingTerminal {
    pub(crate) fn execute_order(&mut self, is_buy: bool) -> Task<Message> {
        match self.order_kind {
            OrderKind::Chase => return self.start_chase(is_buy),
            OrderKind::Twap => return self.start_twap(is_buy),
            OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc => {}
        }

        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let active_symbol = self.active_symbol.clone();
        let sym = self
            .exchange_symbols
            .iter()
            .find(|s| s.key == active_symbol);
        let Some(sym) = sym else {
            self.order_status = Some((
                format!("Symbol '{active_symbol}' not found in exchange metadata"),
                true,
            ));
            return Task::none();
        };
        if let Err(message) = self.validate_exchange_symbol_orderable(sym, "Active") {
            self.order_status = Some((message, true));
            return Task::none();
        }

        let prepared = match self.prepare_order_submission(sym, is_buy) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };
        if prepared.is_outcome {
            self.order_quantity_is_usd = false;
        }

        self.order_status = Some(("Placing order...".into(), false));
        self.pending_order_action = Some(if is_buy {
            PendingOrderAction::Buy
        } else {
            PendingOrderAction::Sell
        });
        let pending_indicator_id = if prepared.order_kind == OrderKind::Market {
            self.add_pending_market_order_placement_indicator(
                self.connected_address.clone().unwrap_or_default(),
                active_symbol,
                prepared.is_buy,
                prepared.size.clone(),
                prepared.price.clone(),
            )
        } else {
            self.add_pending_order_placement_indicator(
                self.connected_address.clone().unwrap_or_default(),
                active_symbol,
                prepared.is_buy,
                prepared.size.clone(),
                prepared.price.clone(),
            )
        };

        Task::perform(
            place_order(
                key.into(),
                prepared.asset,
                prepared.is_buy,
                prepared.price,
                prepared.size,
                prepared.order_kind,
                prepared.reduce_only,
            ),
            move |result| Message::OrderResult {
                pending_indicator_id,
                result: Box::new(result),
            },
        )
    }

    fn prepare_order_submission(
        &self,
        sym: &ExchangeSymbol,
        is_buy: bool,
    ) -> Result<PreparedOrderSubmission, String> {
        self.validate_exchange_symbol_orderable(sym, "Active")?;

        let symbol_key = sym.key.as_str();
        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let is_outcome = sym.market_type == MarketType::Outcome;
        let is_spot_like = Self::market_type_is_spot_like(sym.market_type);

        let raw_qty = match parse_positive_amount(&self.order_quantity) {
            Some(quantity) => quantity,
            None => return Err("Invalid quantity".into()),
        };
        if is_outcome {
            self.validate_outcome_contract_size(raw_qty)?;
        }

        let price = match self.order_kind {
            OrderKind::Limit | OrderKind::LimitIoc => {
                let px = match parse_positive_amount(&self.order_price) {
                    Some(price) => price,
                    None => return Err("Invalid price".into()),
                };
                let rounded = round_price(px, sz_decimals, is_spot_like);
                if is_outcome && let Err(e) = Self::validate_outcome_order_price(rounded) {
                    return Err(e);
                }
                self.validate_order_price_band(symbol_key, rounded)?;
                rounded
            }
            OrderKind::Market => {
                let Some(mid) = self.resolve_mid_for_symbol(symbol_key) else {
                    return Err(format!(
                        "No mid price for {} (tried {})",
                        symbol_key,
                        self.mid_candidates_for_symbol(symbol_key).join(", ")
                    ));
                };
                let rounded = if is_outcome {
                    let slipped =
                        slipped_market_price(mid, is_buy, self.market_slippage_fraction());
                    let clamped = Self::clamp_outcome_market_price(slipped);
                    let rounded = round_price(clamped, sz_decimals, is_spot_like);
                    Self::clamp_outcome_market_price(rounded)
                } else {
                    rounded_market_price(
                        mid,
                        is_buy,
                        self.market_slippage_fraction(),
                        sz_decimals,
                        is_spot_like,
                    )
                };
                if is_outcome && let Err(e) = Self::validate_outcome_order_price(rounded) {
                    return Err(e);
                }
                self.validate_order_price_band(symbol_key, rounded)?;
                rounded
            }
            OrderKind::Chase | OrderKind::Twap => unreachable!("advanced order modes return early"),
        };

        let qty = match order_size_from_quantity_input(
            raw_qty,
            price,
            if is_outcome {
                false
            } else {
                self.order_quantity_is_usd
            },
            sz_decimals,
        ) {
            Some(quantity) => quantity,
            None => return Err("Invalid quantity for asset precision".into()),
        };
        if is_outcome && let Err(e) = self.validate_outcome_contract_size(qty) {
            return Err(e);
        }

        let size_str = float_to_wire(qty);
        let price_str = float_to_wire(price);

        let order_kind = self.order_kind;
        let reduce_only = if is_spot_like {
            false
        } else {
            self.order_reduce_only
        };

        Ok(PreparedOrderSubmission {
            asset,
            is_buy,
            price: price_str,
            size: size_str,
            order_kind,
            reduce_only,
            is_outcome,
        })
    }
}
