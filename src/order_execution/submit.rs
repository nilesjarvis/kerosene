use super::PendingOrderAction;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OrderSurface, PlaceIntent, PreparedExchangeOrder, PriceSource,
    QuantityDenomination, QuantitySource, ReduceOnlySource, place_order_task,
};
use crate::signing::{ExchangeOrderKind, OrderKind};

use iced::Task;

#[cfg(test)]
mod inputs;
#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn execute_order(&mut self, is_buy: bool) -> Task<Message> {
        self.execute_order_with_surface(is_buy, OrderSurface::Ticket)
    }

    pub(crate) fn execute_order_with_surface(
        &mut self,
        is_buy: bool,
        surface: OrderSurface,
    ) -> Task<Message> {
        match self.order_kind {
            OrderKind::Chase => return self.start_chase(is_buy),
            OrderKind::Twap => return self.start_twap(is_buy),
            OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc => {}
        }

        // The disabled-button state in the view is not a submission gate:
        // queued click events can outrun a re-render, and presets/Alfred
        // reach this path without any button state at all.
        if self.pending_order_action.is_some() {
            self.order_status = Some(("Wait for the pending order action to finish".into(), true));
            return Task::none();
        }

        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let exchange_order_kind = match ExchangeOrderKind::try_from(self.order_kind) {
            Ok(kind) => kind,
            Err(message) => {
                self.order_status = Some((message.into(), true));
                return Task::none();
            }
        };
        let active_symbol = self.active_symbol.clone();
        let intent = PlaceIntent {
            surface,
            symbol_key: active_symbol,
            is_buy,
            order_kind: exchange_order_kind,
            price_source: match exchange_order_kind {
                ExchangeOrderKind::Market => PriceSource::MarketWithSlippage {
                    invalid_message: None,
                    usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
                },
                ExchangeOrderKind::Limit | ExchangeOrderKind::LimitIoc => PriceSource::LimitInput {
                    value: self.order_price.clone(),
                    invalid_message: "Invalid price",
                },
            },
            quantity_source: QuantitySource::UserInput {
                value: self.order_quantity.clone(),
                denomination: if self.order_quantity_is_usd {
                    QuantityDenomination::UsdNotional
                } else {
                    QuantityDenomination::Coin
                },
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(self.order_reduce_only),
        };

        let prepared = match self.prepare_place_order(intent) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };
        if prepared.market_type == MarketType::Outcome {
            self.order_quantity_is_usd = false;
        }

        self.submit_prepared_ticket_order(key, prepared)
    }

    fn submit_prepared_ticket_order(
        &mut self,
        key: String,
        prepared: PreparedExchangeOrder,
    ) -> Task<Message> {
        self.order_status = Some(("Placing order...".into(), false));
        self.pending_order_action = Some(if prepared.is_buy {
            PendingOrderAction::Buy
        } else {
            PendingOrderAction::Sell
        });
        let account_address = self.connected_address.clone().unwrap_or_default();
        let pending_indicator_id = if prepared.order_kind == ExchangeOrderKind::Market {
            self.add_pending_market_order_placement_indicator(
                account_address.clone(),
                prepared.symbol_key.clone(),
                prepared.is_buy,
                prepared.size.clone(),
                prepared.price.clone(),
            )
        } else {
            self.add_pending_order_placement_indicator(
                account_address.clone(),
                prepared.symbol_key.clone(),
                prepared.is_buy,
                prepared.size.clone(),
                prepared.price.clone(),
            )
        };

        let (request, context) = prepared.place_request_with_context(&account_address);
        place_order_task(key.into(), request, move |result| Message::OrderResult {
            pending_indicator_id,
            context,
            result: Box::new(result),
        })
    }
}
