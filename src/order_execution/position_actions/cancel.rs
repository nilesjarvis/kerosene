use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::optimistic_updates::{OrderCancellationContext, OrderCancellationResult};
use crate::signing::cancel_order;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn execute_cancel(&mut self, coin: &str, oid: u64) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() {
            self.order_status = Some(("Enter agent key to cancel orders".into(), true));
            return Task::none();
        }
        if self.symbol_key_is_hidden(coin) {
            self.order_status = Some(("Order ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }

        let sym = self.exchange_symbols.iter().find(|s| s.key == coin);
        let Some(sym) = sym else {
            self.order_status = Some((format!("Symbol '{coin}' not found"), true));
            return Task::none();
        };
        let asset = sym.asset_index;
        let account_address = self.connected_address.clone().unwrap_or_default();
        let pending_id = if account_address.is_empty() {
            None
        } else {
            let order = self
                .projected_open_orders()
                .into_iter()
                .find(|row| row.order.coin == coin && row.order.oid == oid)
                .map(|row| row.order.clone());
            order
                .as_ref()
                .and_then(|order| self.add_pending_order_cancellation(&account_address, order))
        };
        let context = OrderCancellationContext {
            account_address,
            symbol: coin.to_string(),
            oid,
            pending_id,
        };

        self.order_status = Some(("Cancelling order...".into(), false));
        Task::perform(cancel_order(key.into(), asset, oid), move |result| {
            Message::CancelResult(Box::new(OrderCancellationResult {
                context: context.clone(),
                result,
            }))
        })
    }
}
