use super::PendingOrderAction;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::config::MarketUniverseConfig;
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OrderSurface, PlaceIntent, PreparedExchangeOrder, PriceSource,
    QuantityDenomination, QuantitySource, ReduceOnlySource, place_order_task,
};
use crate::signing::{ExchangeOrderKind, OrderKind};

use iced::Task;
use std::fmt;
use zeroize::Zeroizing;

#[cfg(test)]
mod inputs;
#[cfg(test)]
mod tests;

pub(crate) struct TicketOrderPlaceIntent {
    pub(crate) surface: OrderSurface,
    pub(crate) symbol_key: String,
    pub(crate) is_buy: bool,
    pub(crate) order_kind: ExchangeOrderKind,
    pub(crate) price_input: String,
    pub(crate) quantity_input: String,
    pub(crate) quantity_is_usd: bool,
    pub(crate) reduce_only: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TicketOrderSubmissionSnapshot {
    pub(crate) order_kind: OrderKind,
    pub(crate) symbol_key: String,
    pub(crate) price_input: String,
    pub(crate) quantity_input: String,
    pub(crate) quantity_is_usd: bool,
    pub(crate) reduce_only: bool,
    pub(crate) market_universe: MarketUniverseConfig,
}

impl fmt::Debug for TicketOrderSubmissionSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TicketOrderSubmissionSnapshot")
            .field("order_kind", &self.order_kind)
            .field("symbol_key", &"<redacted>")
            .field("price_input", &"<redacted>")
            .field("quantity_input", &"<redacted>")
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("reduce_only", &self.reduce_only)
            .field("market_universe", &self.market_universe)
            .finish()
    }
}

impl TradingTerminal {
    pub(crate) fn ticket_order_submission_snapshot(&self) -> TicketOrderSubmissionSnapshot {
        TicketOrderSubmissionSnapshot {
            order_kind: self.order_kind,
            symbol_key: self.active_symbol.clone(),
            price_input: self.order_price.clone(),
            quantity_input: self.order_quantity.clone(),
            quantity_is_usd: self.order_quantity_is_usd,
            reduce_only: self.order_reduce_only,
            market_universe: self.market_universe.clone(),
        }
    }

    fn ticket_order_submission_snapshot_matches(
        &self,
        snapshot: &TicketOrderSubmissionSnapshot,
    ) -> bool {
        self.order_kind == snapshot.order_kind
            && self.active_symbol == snapshot.symbol_key
            && self.order_price == snapshot.price_input
            && self.order_quantity == snapshot.quantity_input
            && self.order_quantity_is_usd == snapshot.quantity_is_usd
            && self.order_reduce_only == snapshot.reduce_only
            && self.market_universe == snapshot.market_universe
    }

    pub(crate) fn execute_order_from_snapshot(
        &mut self,
        is_buy: bool,
        snapshot: TicketOrderSubmissionSnapshot,
    ) -> Task<Message> {
        if !self.ticket_order_submission_snapshot_matches(&snapshot) {
            self.order_status = Some(("Order form changed; review and submit again".into(), true));
            return Task::none();
        }
        if !matches!(
            snapshot.order_kind,
            OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc
        ) {
            self.order_status = Some(("Order form changed; review and submit again".into(), true));
            return Task::none();
        }

        self.execute_order(is_buy)
    }

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
        if self.reject_if_pending_trading_request("placing an order") {
            return Task::none();
        }
        if self.reject_if_account_reconciliation_required("placing an order", "account data") {
            return Task::none();
        }
        if let Some(task) = self.stale_percentage_order_quantity_task("placing an order") {
            return task;
        }

        let _theme = self.theme();
        let Some((key, account_address)) = self.order_signing_context() else {
            return Task::none();
        };

        let exchange_order_kind = match ExchangeOrderKind::try_from(self.order_kind) {
            Ok(kind) => kind,
            Err(message) => {
                self.order_status = Some((message.into(), true));
                return Task::none();
            }
        };
        let intent = Self::ticket_order_place_intent(TicketOrderPlaceIntent {
            surface,
            symbol_key: self.active_symbol.clone(),
            is_buy,
            order_kind: exchange_order_kind,
            price_input: self.order_price.clone(),
            quantity_input: self.order_quantity.clone(),
            quantity_is_usd: self.order_quantity_is_usd,
            reduce_only: self.order_reduce_only,
        });

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

        self.submit_prepared_ticket_order(key, account_address, prepared)
    }

    pub(crate) fn ticket_order_place_intent(input: TicketOrderPlaceIntent) -> PlaceIntent {
        PlaceIntent {
            surface: input.surface,
            symbol_key: input.symbol_key,
            is_buy: input.is_buy,
            order_kind: input.order_kind,
            price_source: match input.order_kind {
                ExchangeOrderKind::Market => PriceSource::MarketWithSlippage {
                    invalid_message: None,
                    usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
                },
                ExchangeOrderKind::Limit | ExchangeOrderKind::LimitIoc => PriceSource::LimitInput {
                    value: input.price_input,
                    invalid_message: "Invalid price",
                },
            },
            quantity_source: QuantitySource::UserInput {
                value: input.quantity_input,
                denomination: if input.quantity_is_usd {
                    QuantityDenomination::UsdNotional
                } else {
                    QuantityDenomination::Coin
                },
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(input.reduce_only),
        }
    }

    fn submit_prepared_ticket_order(
        &mut self,
        key: Zeroizing<String>,
        account_address: String,
        prepared: PreparedExchangeOrder,
    ) -> Task<Message> {
        self.order_status = Some(("Placing order...".into(), false));
        self.pending_order_action = Some(if prepared.is_buy {
            PendingOrderAction::Buy
        } else {
            PendingOrderAction::Sell
        });
        // IOC limit orders are taker orders that never rest, so they project
        // like market orders instead of drawing a provisional resting line.
        let pending_indicator_id = if prepared.order_kind != ExchangeOrderKind::Limit {
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
        place_order_task(key, request, move |result| Message::OrderResult {
            pending_indicator_id,
            context,
            result: Box::new(result),
        })
    }
}
