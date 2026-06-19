use crate::app_state::TradingTerminal;
use crate::helpers::parse_positive_finite_number;
use crate::message::Message;
use crate::order_execution::{
    ModifyIntent, MoveOrderKey, OrderSurface, PendingMoveOrderContext, PreparedModifyOrderResult,
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
    pub(crate) fn handle_move_order(
        &mut self,
        coin: String,
        oid: u64,
        new_price: f64,
    ) -> Task<Message> {
        let _theme = self.theme();
        let move_key = MoveOrderKey::new(coin, oid);
        if self.has_pending_cancel_indicator(oid) {
            self.order_status = Some((
                format!("Move failed: cancel already pending for order {oid}"),
                true,
            ));
            return Task::none();
        }
        if self.pending_move_order_contexts.contains_key(&move_key) {
            self.order_status = Some(("Move already pending for this order".into(), true));
            return Task::none();
        }
        if self.reject_if_pending_trading_request("moving orders") {
            return Task::none();
        }

        let Some((key, account_address)) = self.order_signing_context() else {
            return Task::none();
        };
        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh open orders before moving".into(),
                true,
            ));
            return Task::none();
        }
        if self.reject_if_account_reconciliation_required("moving", "open orders") {
            return Task::none();
        }
        let Some(account_data) = self.account_data_for_order_account(&account_address) else {
            self.order_status = Some((
                "No account data available; refresh before moving".into(),
                true,
            ));
            return Task::none();
        };
        let now_ms = Self::now_ms();
        if !account_data.completeness.open_orders_complete {
            self.order_status = Some((
                "Open orders are incomplete; refresh before moving".into(),
                true,
            ));
            return self.refresh_account_data();
        }
        if !account_data.is_fresh_for_open_order_action_for_symbol(move_key.coin(), now_ms) {
            let age_label = account_data
                .open_order_action_snapshot_age_ms_for_symbol(move_key.coin(), now_ms)
                .map(|age| format!("{}s old", age.div_ceil(1000)))
                .unwrap_or_else(|| "from the future".to_string());
            self.order_status = Some((
                format!("Open orders are stale ({age_label}); refresh before moving orders"),
                true,
            ));
            return self.refresh_account_data();
        }
        let Some(order) = account_data
            .open_orders
            .iter()
            .find(|order| order.oid == oid && order.coin == move_key.coin())
            .cloned()
        else {
            self.order_status = Some(("Order no longer exists".into(), true));
            return Task::none();
        };

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

        let display_coin = self.display_name_for_symbol(&coin);
        self.order_status = Some((
            format!("Moving {} order to ${}...", display_coin, prepared.price),
            false,
        ));
        let Ok(context) = PendingMoveOrderContext::new(account_address.clone(), key) else {
            self.order_status = Some(("Move failed: no agent key".into(), true));
            return Task::none();
        };
        let key = match context.replacement_agent_key(Some(account_address.as_str())) {
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
        self.pending_move_order_contexts
            .insert(move_key.clone(), context);
        self.sync_all_chart_orders();

        modify_order_task(key, prepared, move |r| Message::MoveOrderModifyResult {
            account_address: account_address.clone().into(),
            coin: move_key.coin().to_string(),
            oid,
            pending_indicator_id,
            result: Box::new(r),
        })
    }
}
