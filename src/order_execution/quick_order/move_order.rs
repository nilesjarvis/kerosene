use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{finite_value, positive_finite_value};
use crate::message::Message;
use crate::order_execution::PendingMoveOrderContext;
use crate::signing::{float_to_wire, modify_order, round_price};

use iced::Task;

#[cfg(test)]
mod tests;

fn moved_order_price_wire(
    new_price: f64,
    original_price: f64,
    sz_decimals: u32,
    is_spot: bool,
) -> Option<(f64, String)> {
    let new_price = finite_value(new_price)?;
    let original_price = positive_finite_value(original_price)?;

    let rounded = round_price(new_price, sz_decimals, is_spot);
    let rounded = positive_finite_value(rounded)?;

    let rounded_original = round_price(original_price, sz_decimals, is_spot);
    if (rounded - rounded_original).abs() < 1e-12 {
        return None;
    }

    Some((rounded, float_to_wire(rounded)))
}

fn moved_order_size_wire(size: &str) -> Option<String> {
    let size = size.trim().parse::<f64>().ok()?;
    finite_value(size)
        .filter(|size| *size > 1e-12)
        .map(float_to_wire)
}

fn moved_order_is_buy(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

fn moved_order_reduce_only(
    market_type: MarketType,
    reduce_only: Option<bool>,
) -> Result<bool, &'static str> {
    if TradingTerminal::market_type_is_spot_like(market_type) {
        return Ok(false);
    }
    reduce_only.ok_or(concat!(
        "Move failed: open order reduce-only metadata is unavailable; ",
        "refresh account data before moving this order"
    ))
}

impl TradingTerminal {
    pub(crate) fn handle_move_order(&mut self, oid: u64, new_price: f64) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }
        let Some(account_address) = self.connected_address.clone() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        };
        if self.pending_move_order_contexts.contains_key(&oid) {
            self.order_status = Some(("Move already pending for this order".into(), true));
            return Task::none();
        }

        let order = self
            .account_data
            .as_ref()
            .and_then(|d| d.open_orders.iter().find(|o| o.oid == oid));
        let Some(order) = order else {
            self.order_status = Some(("Order no longer exists".into(), true));
            return Task::none();
        };
        let order = order.clone();

        let coin = order.coin.clone();
        if self.symbol_key_is_hidden(&coin) {
            self.order_status = Some(("Order ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }
        let Some(is_buy) = moved_order_is_buy(&order.side) else {
            self.order_status = Some(("Move failed: open order has invalid side".into(), true));
            return Task::none();
        };
        let Some(size) = moved_order_size_wire(&order.sz) else {
            self.order_status = Some(("Move failed: open order has invalid size".into(), true));
            return Task::none();
        };
        let Ok(original_px) = order.limit_px.parse::<f64>() else {
            self.order_status = Some(("Move failed: open order has invalid price".into(), true));
            return Task::none();
        };

        let sym = self.exchange_symbols.iter().find(|s| s.key == coin);
        let Some(sym) = sym else {
            self.order_status = Some((format!("Symbol '{}' not found", coin), true));
            return Task::none();
        };
        if let Err(message) = self.validate_exchange_symbol_orderable(sym, "Order") {
            self.order_status = Some((message, true));
            return Task::none();
        }
        if sym.market_type == MarketType::Outcome {
            let raw_size = order.sz.trim().parse::<f64>().unwrap_or(f64::NAN);
            if let Err(e) = self.validate_outcome_contract_size(raw_size) {
                self.order_status = Some((format!("Move failed: {e}"), true));
                return Task::none();
            }
        }
        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let reduce_only = match moved_order_reduce_only(sym.market_type, order.reduce_only) {
            Ok(reduce_only) => reduce_only,
            Err(message) => {
                self.order_status = Some((message.to_string(), true));
                return Task::none();
            }
        };

        let is_spot = Self::market_type_is_spot_like(sym.market_type);
        let Some((rounded_price, new_price_str)) =
            moved_order_price_wire(new_price, original_px, sz_decimals, is_spot)
        else {
            if positive_finite_value(original_px).is_none() {
                self.order_status =
                    Some(("Move failed: open order has invalid price".into(), true));
            }
            return Task::none();
        };
        if sym.market_type == MarketType::Outcome
            && let Err(e) = Self::validate_outcome_order_price(rounded_price)
        {
            self.order_status = Some((e, true));
            return Task::none();
        }
        if let Err(e) = self.validate_order_price_band(&coin, rounded_price) {
            self.order_status = Some((e, true));
            return Task::none();
        }

        self.order_status = Some((
            format!("Moving {} order to ${}...", coin, new_price_str),
            false,
        ));
        let Ok(context) = PendingMoveOrderContext::new(account_address.clone(), key) else {
            self.order_status = Some(("Move failed: no agent key".into(), true));
            return Task::none();
        };
        let key = match context.replacement_agent_key(self.connected_address.as_deref()) {
            Ok(key) => key,
            Err(error) => {
                self.order_status = Some((error.status_text().into(), true));
                return Task::none();
            }
        };
        let pending_indicator_id = self.add_pending_order_modification_indicator(
            account_address.clone(),
            &order,
            new_price_str.clone(),
        );
        self.pending_move_order_contexts.insert(oid, context);
        self.sync_all_chart_orders();

        Task::perform(
            modify_order(key, oid, asset, is_buy, new_price_str, size, reduce_only),
            move |r| Message::MoveOrderModifyResult {
                oid,
                pending_indicator_id,
                result: Box::new(r),
            },
        )
    }
}
