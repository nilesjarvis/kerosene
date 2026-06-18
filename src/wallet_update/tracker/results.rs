use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::wallet_state::{
    WALLET_TRACKER_CORE_ERROR_BACKOFF_MS, WALLET_TRACKER_ORDER_ERROR_BACKOFF_MS,
};
use iced::Task;

impl TradingTerminal {
    pub(super) fn apply_wallet_tracker_results(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WalletTrackerLoaded(address, context, result) => {
                let address = address.into_string();
                if !self.read_data_request_context_is_current(context) {
                    self.clear_stale_wallet_tracker_core_loading(&address, context);
                    return Task::none();
                }
                self.apply_wallet_tracker_snapshot_result(address, *result);
            }
            Message::WalletTrackerBatchLoaded(context, results) => {
                let results = results.into_vec();
                if !self.read_data_request_context_is_current(context) {
                    for (address, _) in results {
                        self.clear_stale_wallet_tracker_core_loading(&address, context);
                    }
                    return Task::none();
                }
                for (address, result) in results {
                    self.apply_wallet_tracker_snapshot_result(address, result);
                }
            }
            Message::WalletTrackerOrdersLoaded(address, context, result) => {
                let address = address.into_string();
                if !self.read_data_request_context_is_current(context) {
                    self.clear_stale_wallet_tracker_order_loading(&address, context);
                    return Task::none();
                }
                if !self.wallet_tracker.tracked_addresses.contains(&address) {
                    return Task::none();
                }
                let row = self.wallet_tracker.rows.entry(address).or_default();
                row.order_loading = false;
                row.order_loading_context = None;
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

    fn apply_wallet_tracker_snapshot_result(
        &mut self,
        address: String,
        result: Result<crate::account::WalletTrackerSnapshot, String>,
    ) {
        if !self.wallet_tracker.tracked_addresses.contains(&address) {
            self.wallet_tracker.rows.remove(&address);
            return;
        }
        let row = self.wallet_tracker.rows.entry(address).or_default();
        row.loading = false;
        row.loading_context = None;
        match result {
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

    fn clear_stale_wallet_tracker_core_loading(
        &mut self,
        address: &str,
        context: crate::read_data_provider::ReadDataRequestContext,
    ) {
        let cleared = if let Some(row) = self.wallet_tracker.rows.get_mut(address) {
            if row.loading && row.loading_context == Some(context) {
                row.loading = false;
                row.loading_context = None;
                true
            } else {
                false
            }
        } else {
            false
        };

        if cleared {
            self.queue_wallet_tracker_core_refresh(address.to_string());
        }
    }

    fn clear_stale_wallet_tracker_order_loading(
        &mut self,
        address: &str,
        context: crate::read_data_provider::ReadDataRequestContext,
    ) {
        let cleared = if let Some(row) = self.wallet_tracker.rows.get_mut(address) {
            if row.order_loading && row.order_loading_context == Some(context) {
                row.order_loading = false;
                row.order_loading_context = None;
                true
            } else {
                false
            }
        } else {
            false
        };

        if cleared {
            self.queue_wallet_tracker_order_refresh(address.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::WalletTrackerSnapshot;
    use crate::config::ReadDataProvider;
    use crate::read_data_provider::ReadDataRequestContext;

    const TEST_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    fn snapshot() -> WalletTrackerSnapshot {
        WalletTrackerSnapshot {
            equity: Some(100.0),
            withdrawable: Some(50.0),
            unrealized_pnl: Some(1.0),
            margin_used_pct: Some(0.1),
            open_trade_count: Some(1),
            open_order_count: 2,
            long_exposure: Some(10.0),
            short_exposure: Some(0.0),
        }
    }

    fn stale_context() -> ReadDataRequestContext {
        ReadDataRequestContext {
            provider: ReadDataProvider::Hydromancer,
            read_data_provider_generation: 0,
            hydromancer_key_generation: 1,
        }
    }

    fn terminal_with_stale_hydromancer_context() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_key_generation = 2;
        terminal
            .wallet_tracker
            .tracked_addresses
            .push(TEST_ADDRESS.to_string());
        terminal
    }

    #[test]
    fn stale_hydromancer_context_clears_and_requeues_wallet_tracker_snapshot() {
        let mut terminal = terminal_with_stale_hydromancer_context();
        let stale_context = stale_context();
        terminal
            .wallet_tracker
            .rows
            .entry(TEST_ADDRESS.to_string())
            .or_default()
            .loading = true;
        terminal
            .wallet_tracker
            .rows
            .get_mut(TEST_ADDRESS)
            .expect("tracker row")
            .loading_context = Some(stale_context);

        let _task = terminal.apply_wallet_tracker_results(Message::WalletTrackerLoaded(
            TEST_ADDRESS.to_string().into(),
            stale_context,
            Box::new(Ok(snapshot())),
        ));

        let row = terminal
            .wallet_tracker
            .rows
            .get(TEST_ADDRESS)
            .expect("tracker row");
        assert!(!row.loading);
        assert_eq!(row.loading_context, None);
        assert!(row.snapshot.is_none());
        assert_eq!(
            terminal.wallet_tracker.core_refresh_queue,
            vec![TEST_ADDRESS.to_string()]
        );
        assert_eq!(
            terminal.wallet_tracker_next_core_address(TradingTerminal::now_ms()),
            Some(TEST_ADDRESS.to_string())
        );
    }

    #[test]
    fn stale_hydromancer_context_clears_and_requeues_wallet_tracker_batch_or_orders() {
        let mut terminal = terminal_with_stale_hydromancer_context();
        let stale_context = stale_context();
        let row = terminal
            .wallet_tracker
            .rows
            .entry(TEST_ADDRESS.to_string())
            .or_default();
        row.loading = true;
        row.loading_context = Some(stale_context);
        row.order_loading = true;
        row.order_loading_context = Some(stale_context);
        row.snapshot = Some(snapshot());

        let _task = terminal.apply_wallet_tracker_results(Message::WalletTrackerBatchLoaded(
            stale_context,
            vec![(TEST_ADDRESS.to_string(), Ok(snapshot()))].into(),
        ));
        let _task = terminal.apply_wallet_tracker_results(Message::WalletTrackerOrdersLoaded(
            TEST_ADDRESS.to_string().into(),
            stale_context,
            Box::new(Ok(7)),
        ));

        let row = terminal
            .wallet_tracker
            .rows
            .get(TEST_ADDRESS)
            .expect("tracker row");
        assert!(!row.loading);
        assert_eq!(row.loading_context, None);
        assert!(!row.order_loading);
        assert_eq!(row.order_loading_context, None);
        assert!(row.snapshot.is_some());
        assert_eq!(row.open_order_count, None);
        assert_eq!(
            terminal.wallet_tracker.core_refresh_queue,
            vec![TEST_ADDRESS.to_string()]
        );
        assert_eq!(
            terminal.wallet_tracker.order_refresh_queue,
            vec![TEST_ADDRESS.to_string()]
        );
        assert_eq!(
            terminal.wallet_tracker_next_core_address(TradingTerminal::now_ms()),
            Some(TEST_ADDRESS.to_string())
        );
        assert_eq!(
            terminal.wallet_tracker_next_order_address(TradingTerminal::now_ms()),
            Some(TEST_ADDRESS.to_string())
        );
    }

    #[test]
    fn stale_hydromancer_context_does_not_clear_newer_wallet_tracker_request() {
        let mut terminal = terminal_with_stale_hydromancer_context();
        let stale_context = stale_context();
        let current_context = ReadDataRequestContext {
            provider: ReadDataProvider::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: 2,
        };
        let row = terminal
            .wallet_tracker
            .rows
            .entry(TEST_ADDRESS.to_string())
            .or_default();
        row.loading = true;
        row.loading_context = Some(current_context);
        row.order_loading = true;
        row.order_loading_context = Some(current_context);

        let _task = terminal.apply_wallet_tracker_results(Message::WalletTrackerLoaded(
            TEST_ADDRESS.to_string().into(),
            stale_context,
            Box::new(Ok(snapshot())),
        ));
        let _task = terminal.apply_wallet_tracker_results(Message::WalletTrackerOrdersLoaded(
            TEST_ADDRESS.to_string().into(),
            stale_context,
            Box::new(Ok(7)),
        ));

        let row = terminal
            .wallet_tracker
            .rows
            .get(TEST_ADDRESS)
            .expect("tracker row");
        assert!(row.loading);
        assert_eq!(row.loading_context, Some(current_context));
        assert!(row.order_loading);
        assert_eq!(row.order_loading_context, Some(current_context));
        assert!(terminal.wallet_tracker.core_refresh_queue.is_empty());
        assert!(terminal.wallet_tracker.order_refresh_queue.is_empty());
    }
}
