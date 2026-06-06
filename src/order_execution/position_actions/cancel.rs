use crate::app_state::TradingTerminal;
use crate::message::Message;
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
        let pending_indicator_id = self.connected_address.clone().and_then(|account_address| {
            let order = self
                .account_data
                .as_ref()
                .and_then(|data| data.open_orders.iter().find(|order| order.oid == oid))
                .cloned()?;
            self.add_pending_order_cancellation_indicator(account_address, &order)
        });

        self.order_status = Some(("Cancelling order...".into(), false));
        Task::perform(cancel_order(key.into(), asset, oid), move |result| {
            Message::CancelResult {
                pending_indicator_id,
                result: Box::new(result),
            }
        })
    }
}
