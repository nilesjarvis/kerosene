use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::wallet_state::{
    WALLET_TRACKER_CORE_ERROR_BACKOFF_MS, WALLET_TRACKER_ORDER_ERROR_BACKOFF_MS,
};
use iced::Task;

impl TradingTerminal {
    pub(super) fn apply_wallet_tracker_results(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WalletTrackerLoaded(address, result) => {
                if !self.wallet_tracker.tracked_addresses.contains(&address) {
                    self.wallet_tracker.rows.remove(&address);
                    return Task::none();
                }
                let row = self.wallet_tracker.rows.entry(address).or_default();
                row.loading = false;
                match *result {
                    Ok(mut data) => {
                        if let Some(open_order_count) = row.open_order_count {
                            data.open_order_count = open_order_count;
                        }
                        row.error = None;
                        row.next_core_retry_ms = None;
                        row.last_updated_ms = Some(Self::now_ms());
                        row.snapshot = Some(data);
                    }
                    Err(e) => {
                        row.error = Some(e);
                        row.next_core_retry_ms =
                            Some(Self::now_ms() + WALLET_TRACKER_CORE_ERROR_BACKOFF_MS);
                    }
                }
            }
            Message::WalletTrackerOrdersLoaded(address, result) => {
                if !self.wallet_tracker.tracked_addresses.contains(&address) {
                    return Task::none();
                }
                let row = self.wallet_tracker.rows.entry(address).or_default();
                row.order_loading = false;
                match *result {
                    Ok(open_order_count) => {
                        row.open_order_count = Some(open_order_count);
                        row.order_error = None;
                        row.next_order_retry_ms = None;
                        row.orders_last_updated_ms = Some(Self::now_ms());
                        if let Some(snapshot) = row.snapshot.as_mut() {
                            snapshot.open_order_count = open_order_count;
                        }
                    }
                    Err(e) => {
                        row.order_error = Some(e);
                        row.next_order_retry_ms =
                            Some(Self::now_ms() + WALLET_TRACKER_ORDER_ERROR_BACKOFF_MS);
                    }
                }
            }
            _ => {}
        }

        Task::none()
    }
}
