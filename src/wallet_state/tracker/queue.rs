use crate::app_state::TradingTerminal;

impl TradingTerminal {
    pub(crate) fn queue_wallet_tracker_core_refresh(&mut self, address: String) {
        if self.wallet_tracker.tracked_addresses.contains(&address)
            && !self
                .wallet_tracker
                .rows
                .get(&address)
                .is_some_and(|row| row.loading)
            && !self
                .wallet_tracker
                .core_refresh_queue
                .iter()
                .any(|queued| queued == &address)
        {
            self.wallet_tracker.core_refresh_queue.push(address);
        }
    }

    pub(crate) fn queue_wallet_tracker_core_refresh_all(&mut self) {
        self.wallet_tracker.core_refresh_queue.clear();
        let addresses = self.wallet_tracker.tracked_addresses.clone();
        for address in addresses {
            self.queue_wallet_tracker_core_refresh(address);
        }
    }

    pub(crate) fn queue_wallet_tracker_order_refresh(&mut self, address: String) {
        if self.wallet_tracker.tracked_addresses.contains(&address)
            && !self
                .wallet_tracker
                .rows
                .get(&address)
                .is_some_and(|row| row.order_loading)
            && !self
                .wallet_tracker
                .order_refresh_queue
                .iter()
                .any(|queued| queued == &address)
        {
            self.wallet_tracker.order_refresh_queue.push(address);
        }
    }
}
