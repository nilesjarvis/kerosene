use super::super::pricing::rounded_market_price;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::signing::{OrderKind, float_to_wire, place_order, round_price};

use iced::Task;

#[cfg(test)]
mod tests;

fn quick_order_quantity_wire(input: &str) -> Option<String> {
    let quantity = input.trim().parse::<f64>().ok()?;
    if quantity.is_finite() && quantity > 1e-12 {
        Some(float_to_wire(quantity))
    } else {
        None
    }
}

fn quick_order_limit_price_wire(
    price: f64,
    sz_decimals: u32,
    is_spot: bool,
) -> Option<(f64, String)> {
    if !price.is_finite() || price <= 0.0 {
        return None;
    }

    let rounded = round_price(price, sz_decimals, is_spot);
    if rounded.is_finite() && rounded > 0.0 {
        Some((rounded, float_to_wire(rounded)))
    } else {
        None
    }
}

fn quick_order_market_price_wire(
    mid: f64,
    is_buy: bool,
    slippage: f64,
    sz_decimals: u32,
    is_spot: bool,
) -> Option<(f64, String)> {
    let rounded = rounded_market_price(mid, is_buy, slippage, sz_decimals, is_spot);
    if rounded.is_finite() && rounded > 0.0 {
        Some((rounded, float_to_wire(rounded)))
    } else {
        None
    }
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

        let form = self
            .charts
            .get_mut(&chart_id)
            .and_then(|inst| inst.quick_order.take());
        let Some(form) = form else {
            return Task::none();
        };

        let size = match quick_order_quantity_wire(&form.quantity) {
            Some(size) => size,
            None => {
                self.order_status = Some(("Invalid quantity".into(), true));
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.quick_order = Some(form);
                }
                return Task::none();
            }
        };

        let chart_symbol = self
            .charts
            .get(&chart_id)
            .map(|inst| inst.symbol.clone())
            .unwrap_or_default();
        if self.is_ticker_muted(&chart_symbol) {
            self.order_status = Some(("Chart ticker is muted in Settings > Risk".into(), true));
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.quick_order = Some(form);
            }
            return Task::none();
        }
        let sym = self.exchange_symbols.iter().find(|s| s.key == chart_symbol);
        let Some(sym) = sym else {
            self.order_status = Some((format!("Symbol '{}' not found", chart_symbol), true));
            return Task::none();
        };
        if sym.market_type == MarketType::Outcome {
            self.outcome_read_only_status("trading");
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.quick_order = Some(form);
            }
            return Task::none();
        }

        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let is_spot = sym.market_type == MarketType::Spot;

        let (order_kind, price) = if form.is_limit {
            let Some((rounded, price)) =
                quick_order_limit_price_wire(form.price, sz_decimals, is_spot)
            else {
                self.order_status = Some(("Invalid price".into(), true));
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.quick_order = Some(form);
                }
                return Task::none();
            };
            if let Err(e) = self.validate_order_price_band(&chart_symbol, rounded) {
                self.order_status = Some((e, true));
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.quick_order = Some(form);
                }
                return Task::none();
            }
            (OrderKind::Limit, price)
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
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.quick_order = Some(form);
                }
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
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.quick_order = Some(form);
                }
                return Task::none();
            };
            if let Err(e) = self.validate_order_price_band(&chart_symbol, rounded) {
                self.order_status = Some((e, true));
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.quick_order = Some(form);
                }
                return Task::none();
            }
            (OrderKind::Market, price)
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
            |r| Message::QuickOrderResult(Box::new(r)),
        )
    }
}
