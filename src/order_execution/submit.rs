use super::PendingOrderAction;
use super::pricing::{rounded_market_price, slipped_market_price};
use super::sizing::order_size_from_quantity_input;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{OrderKind, float_to_wire, place_order, round_price};

use iced::Task;

mod inputs;

use inputs::parse_positive_amount;

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
        if self.symbol_key_is_hidden(&self.active_symbol) {
            self.order_status = Some(("Active ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }

        let sym = self
            .exchange_symbols
            .iter()
            .find(|s| s.key == self.active_symbol);
        let Some(sym) = sym else {
            self.order_status = Some((
                format!(
                    "Symbol '{}' not found in exchange metadata",
                    self.active_symbol
                ),
                true,
            ));
            return Task::none();
        };
        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let is_outcome = sym.market_type == MarketType::Outcome;
        let is_spot_like = Self::market_type_is_spot_like(sym.market_type);

        let raw_qty = match parse_positive_amount(&self.order_quantity) {
            Some(quantity) => quantity,
            None => {
                self.order_status = Some(("Invalid quantity".into(), true));
                return Task::none();
            }
        };
        if is_outcome {
            if let Err(e) = self.validate_outcome_contract_size(raw_qty) {
                self.order_status = Some((e, true));
                return Task::none();
            }
            self.order_quantity_is_usd = false;
        }

        let price = match self.order_kind {
            OrderKind::Limit | OrderKind::LimitIoc => {
                let px = match parse_positive_amount(&self.order_price) {
                    Some(price) => price,
                    None => {
                        self.order_status = Some(("Invalid price".into(), true));
                        return Task::none();
                    }
                };
                let rounded = round_price(px, sz_decimals, is_spot_like);
                if is_outcome && let Err(e) = Self::validate_outcome_order_price(rounded) {
                    self.order_status = Some((e, true));
                    return Task::none();
                }
                if let Err(e) = self.validate_order_price_band(&self.active_symbol, rounded) {
                    self.order_status = Some((e, true));
                    return Task::none();
                }
                rounded
            }
            OrderKind::Market => {
                let Some(mid) = self.resolve_mid_for_symbol(&self.active_symbol) else {
                    self.order_status = Some((
                        format!(
                            "No mid price for {} (tried {})",
                            self.active_symbol,
                            self.mid_candidates_for_symbol(&self.active_symbol)
                                .join(", ")
                        ),
                        true,
                    ));
                    return Task::none();
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
                    self.order_status = Some((e, true));
                    return Task::none();
                }
                if let Err(e) = self.validate_order_price_band(&self.active_symbol, rounded) {
                    self.order_status = Some((e, true));
                    return Task::none();
                }
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
            None => {
                self.order_status = Some(("Invalid quantity for asset precision".into(), true));
                return Task::none();
            }
        };
        if is_outcome && let Err(e) = self.validate_outcome_contract_size(qty) {
            self.order_status = Some((e, true));
            return Task::none();
        }

        let size_str = float_to_wire(qty);
        let price_str = float_to_wire(price);

        let order_kind = self.order_kind;
        let reduce_only = if is_spot_like {
            false
        } else {
            self.order_reduce_only
        };
        self.order_status = Some(("Placing order...".into(), false));
        self.pending_order_action = Some(if is_buy {
            PendingOrderAction::Buy
        } else {
            PendingOrderAction::Sell
        });

        Task::perform(
            place_order(
                key.into(),
                asset,
                is_buy,
                price_str,
                size_str,
                order_kind,
                reduce_only,
            ),
            |r| Message::OrderResult(Box::new(r)),
        )
    }
}
