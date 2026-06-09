use crate::app_state::TradingTerminal;
use crate::helpers::parse_positive_finite_number;
use crate::message::Message;
use crate::order_execution::{
    ModifyIntent, OrderSurface, PendingMoveOrderContext, PreparedModifyOrderResult,
    modify_order_task,
};

use iced::Task;

#[cfg(test)]
mod tests;

fn moved_order_is_buy(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

fn move_order_wire_is_supported(order: &crate::account::OpenOrder) -> Result<(), &'static str> {
    if order.is_trigger == Some(true)
        || order
            .trigger_px
            .as_deref()
            .and_then(parse_positive_finite_number)
            .is_some()
    {
        return Err("Move failed: trigger orders cannot be moved safely yet");
    }
    if order
        .order_type
        .as_deref()
        .is_some_and(|kind| !kind.eq_ignore_ascii_case("limit"))
    {
        return Err("Move failed: order type cannot be moved safely yet");
    }
    if order
        .tif
        .as_deref()
        .is_some_and(|tif| !tif.eq_ignore_ascii_case("Gtc"))
    {
        return Err("Move failed: non-GTC orders cannot be moved safely yet");
    }
    Ok(())
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
        if let Err(message) = move_order_wire_is_supported(&order) {
            self.order_status = Some((message.into(), true));
            return Task::none();
        }
        if self.symbol_key_is_hidden(&coin) {
            self.order_status = Some(("Order ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }
        let Some(is_buy) = moved_order_is_buy(&order.side) else {
            self.order_status = Some(("Move failed: open order has invalid side".into(), true));
            return Task::none();
        };

        let prepared = match self.prepare_modify_order(ModifyIntent {
            surface: OrderSurface::Move,
            symbol_key: coin.clone(),
            oid,
            is_buy,
            new_price,
            original_price: order.limit_px.clone(),
            size: order.sz.clone(),
            invalid_size_message: "Move failed: open order has invalid size",
            reduce_only: order.reduce_only,
            reduce_only_missing_message: concat!(
                "Move failed: open order reduce-only metadata is unavailable; ",
                "refresh account data before moving this order"
            ),
            invalid_price_message: "Move failed: open order has invalid price",
        }) {
            Ok(PreparedModifyOrderResult::Prepared(prepared)) => prepared,
            Ok(PreparedModifyOrderResult::NoPriceChange) => {
                return Task::none();
            }
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };

        self.order_status = Some((
            format!("Moving {} order to ${}...", coin, prepared.price),
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
            prepared.price.clone(),
        );
        self.pending_move_order_contexts.insert(oid, context);
        self.sync_all_chart_orders();

        modify_order_task(key, prepared, move |r| Message::MoveOrderModifyResult {
            oid,
            pending_indicator_id,
            result: Box::new(r),
        })
    }
}
