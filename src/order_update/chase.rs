mod cancel;
mod modify;
mod resting;
mod result;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    fn check_chase_order_status(
        &mut self,
        chase_id: u64,
        oid: u64,
        status: impl Into<String>,
    ) -> Task<Message> {
        let status = status.into();
        let can_refresh_chase_account = self.chase_orders.get(&chase_id).is_some_and(|chase| {
            self.connected_address.as_deref() == Some(chase.account_address.as_str())
        });
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.current_oid = Some(oid);
            chase.pending_op = None;
            chase.missing_open_order_refresh_requested = true;
        }
        if can_refresh_chase_account {
            self.order_status = Some((status, false));
            self.refresh_account_data()
        } else {
            self.order_status = Some((
                format!("{status}; reconnect to verify the previous account's open orders"),
                true,
            ));
            self.remove_chase_order(chase_id);
            Task::none()
        }
    }
}
