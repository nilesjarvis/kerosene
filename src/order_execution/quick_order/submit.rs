use super::super::pricing::rounded_market_price;
use super::super::sizing::order_size_from_quantity_input;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::helpers::{parse_number, positive_finite_value};
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use crate::signing::{OrderKind, float_to_wire, place_order, round_price};

use iced::Task;

#[cfg(test)]
mod tests;

fn quick_order_size_wire(
    input: &str,
    quantity_is_usd: bool,
    reference_price: f64,
    sz_decimals: u32,
) -> Option<String> {
    let quantity = parse_number(input)?;
    order_size_from_quantity_input(quantity, reference_price, quantity_is_usd, sz_decimals)
        .map(float_to_wire)
}

fn quick_order_limit_price_wire(
    price: f64,
    sz_decimals: u32,
    is_spot: bool,
) -> Option<(f64, String)> {
    let price = positive_finite_value(price)?;

    let rounded = round_price(price, sz_decimals, is_spot);
    positive_finite_value(rounded).map(|rounded| (rounded, float_to_wire(rounded)))
}

fn quick_order_market_price_wire(
    mid: f64,
    is_buy: bool,
    slippage: f64,
    sz_decimals: u32,
    is_spot: bool,
) -> Option<(f64, String)> {
    let rounded = rounded_market_price(mid, is_buy, slippage, sz_decimals, is_spot);
    positive_finite_value(rounded).map(|rounded| (rounded, float_to_wire(rounded)))
}

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
        if self.symbol_key_is_hidden(&chart_symbol) {
            self.order_status = Some(("Chart ticker is hidden in Settings > Risk".into(), true));
            self.restore_quick_order_form(chart_id, form, quick_order_surface);
            return Task::none();
        }
        let sym = self.exchange_symbols.iter().find(|s| s.key == chart_symbol);
        let Some(sym) = sym else {
            self.order_status = Some((format!("Symbol '{}' not found", chart_symbol), true));
            self.restore_quick_order_form(chart_id, form, quick_order_surface);
            return Task::none();
        };
        if sym.market_type == MarketType::Outcome {
            self.outcome_read_only_status("trading");
            self.restore_quick_order_form(chart_id, form, quick_order_surface);
            return Task::none();
        }

        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let is_spot = sym.market_type == MarketType::Spot;

        let (order_kind, price, reference_price) = if form.is_limit {
            let Some((rounded, price)) =
                quick_order_limit_price_wire(form.price, sz_decimals, is_spot)
            else {
                self.order_status = Some(("Invalid price".into(), true));
                self.restore_quick_order_form(chart_id, form, quick_order_surface);
                return Task::none();
            };
            if let Err(e) = self.validate_order_price_band(&chart_symbol, rounded) {
                self.order_status = Some((e, true));
                self.restore_quick_order_form(chart_id, form, quick_order_surface);
                return Task::none();
            }
            (OrderKind::Limit, price, rounded)
        } else {
            let Some(mid) = self.resolve_mid_for_symbol(&chart_symbol) else {
                self.order_status = Some((
                    format!(
                        "No mid price for {} (tried {})",
                        chart_symbol,
                        self.mid_candidates_for_symbol(&chart_symbol).join(", ")
                    ),
                    true,
                ));
                self.restore_quick_order_form(chart_id, form, quick_order_surface);
                return Task::none();
            };
            let Some((rounded, price)) = quick_order_market_price_wire(
                mid,
                is_buy,
                self.market_slippage_fraction(),
                sz_decimals,
                is_spot,
            ) else {
                self.order_status = Some(("Invalid market price".into(), true));
                self.restore_quick_order_form(chart_id, form, quick_order_surface);
                return Task::none();
            };
            if let Err(e) = self.validate_order_price_band(&chart_symbol, rounded) {
                self.order_status = Some((e, true));
                self.restore_quick_order_form(chart_id, form, quick_order_surface);
                return Task::none();
            }
            (OrderKind::Market, price, mid)
        };

        let size = match quick_order_size_wire(
            &form.quantity,
            form.quantity_is_usd,
            reference_price,
            sz_decimals,
        ) {
            Some(size) => size,
            None => {
                self.order_status = Some(("Invalid quantity for asset precision".into(), true));
                self.restore_quick_order_form(chart_id, form, quick_order_surface);
                return Task::none();
            }
        };

        let reduce_only = if is_spot {
            false
        } else {
            self.order_reduce_only
        };
        let side_str = if is_buy { "BUY" } else { "SELL" };
        let kind_str = if form.is_limit { "limit" } else { "market" };
        self.order_status = Some((
            format!("Placing {kind_str} {side_str} {size} {chart_symbol}..."),
            false,
        ));
        let pending_indicator_id = self.add_pending_order_placement_indicator(
            self.connected_address.clone().unwrap_or_default(),
            chart_symbol,
            is_buy,
            size.clone(),
            price.clone(),
        );

        Task::perform(
            place_order(
                key.into(),
                asset,
                is_buy,
                price,
                size,
                order_kind,
                reduce_only,
            ),
            move |result| Message::QuickOrderResult {
                pending_indicator_id,
                result: Box::new(result),
            },
        )
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
