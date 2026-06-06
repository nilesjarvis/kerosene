use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OrderSurface, PlaceIntent, PreparedExchangeOrder, PriceSource,
    QuantityDenomination, QuantitySource, QuickOrderForm, ReduceOnlySource, place_order_task,
};
use crate::signing::ExchangeOrderKind;

use iced::Task;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn handle_submit_quick_order(
        &mut self,
        chart_id: ChartId,
        is_buy: bool,
    ) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let quick_order_surface = self.chart_quick_order_surface.remove(&chart_id);
        let form = self
            .charts
            .get_mut(&chart_id)
            .and_then(|inst| inst.take_quick_order());
        let Some(form) = form else {
            return Task::none();
        };

        let chart_symbol = self
            .charts
            .get(&chart_id)
            .map(|inst| inst.symbol.clone())
            .unwrap_or_default();
        let order_kind = if form.is_limit {
            ExchangeOrderKind::Limit
        } else {
            ExchangeOrderKind::Market
        };
        let intent = PlaceIntent {
            surface: OrderSurface::QuickOrder,
            symbol_key: chart_symbol,
            is_buy,
            order_kind,
            price_source: if form.is_limit {
                PriceSource::LimitInput {
                    value: form.price.to_string(),
                    invalid_message: "Invalid price",
                }
            } else {
                PriceSource::MarketWithSlippage {
                    invalid_message: Some("Invalid market price"),
                    usd_size_reference: MarketUsdSizeReference::Mid,
                }
            },
            quantity_source: QuantitySource::UserInput {
                value: form.quantity.clone(),
                denomination: if form.quantity_is_usd {
                    QuantityDenomination::UsdNotional
                } else {
                    QuantityDenomination::Coin
                },
                invalid_message: "Invalid quantity for asset precision",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(self.order_reduce_only),
        };
        let prepared = match self.prepare_place_order(intent) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                self.restore_quick_order_form(chart_id, form, quick_order_surface);
                return Task::none();
            }
        };

        self.submit_prepared_quick_order(key, prepared, form.is_limit)
    }

    fn submit_prepared_quick_order(
        &mut self,
        key: String,
        prepared: PreparedExchangeOrder,
        is_limit: bool,
    ) -> Task<Message> {
        let side_str = if prepared.is_buy { "BUY" } else { "SELL" };
        let kind_str = if is_limit { "limit" } else { "market" };
        self.order_status = Some((
            format!(
                "Placing {kind_str} {side_str} {} {}...",
                prepared.size, prepared.symbol_key
            ),
            false,
        ));
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
        place_order_task(key.into(), request, move |result| {
            Message::QuickOrderResult {
                pending_indicator_id,
                context,
                result: Box::new(result),
            }
        })
    }

    fn restore_quick_order_form(
        &mut self,
        chart_id: ChartId,
        form: QuickOrderForm,
        surface_id: Option<ChartSurfaceId>,
    ) {
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.set_quick_order(form);
        }
        if let Some(surface_id) = surface_id {
            self.chart_quick_order_surface.insert(chart_id, surface_id);
        }
    }
}
