use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::{CancelIntent, OrderSurface, cancel_order_task};

use iced::Task;

impl TradingTerminal {
    pub(crate) fn execute_cancel(&mut self, coin: &str, oid: u64) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() {
            self.order_status = Some(("Enter agent key to cancel orders".into(), true));
            return Task::none();
        }
        let prepared = match self.prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: coin.to_string(),
            oid,
        }) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };
        let pending_indicator_id = self.connected_address.clone().and_then(|account_address| {
            let order = self
                .account_data
                .as_ref()
                .and_then(|data| data.open_orders.iter().find(|order| order.oid == oid))
                .cloned()?;
            self.add_pending_order_cancellation_indicator(account_address, &order)
        });

        self.order_status = Some(("Cancelling order...".into(), false));
        cancel_order_task(key.into(), prepared.asset, prepared.oid, move |result| {
            Message::CancelResult {
                pending_indicator_id,
                result: Box::new(result),
            }
        })
    }
}
