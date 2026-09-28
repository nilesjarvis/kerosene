use crate::app_state::TradingTerminal;

impl TradingTerminal {
    pub(crate) fn queue_wallet_tracker_core_refresh(&mut self, address: String) {
        let tracker = &mut self.wallet_tracker;
        if tracker.tracked_addresses.contains(&address)
            && !tracker.rows.get(&address).is_some_and(|row| row.loading)
            && !tracker.core_refresh_queue.contains(&address)
        {
            tracker.core_refresh_queue.push(address);
        }
    }

    pub(crate) fn queue_wallet_tracker_core_refresh_all(&mut self) {
        let tracker = &mut self.wallet_tracker;
        tracker.core_refresh_queue.clear();
        for address in &tracker.tracked_addresses {
            if !tracker.rows.get(address).is_some_and(|row| row.loading)
                && !tracker.core_refresh_queue.contains(address)
            {
                tracker.core_refresh_queue.push(address.clone());
            }
        }
    }

    pub(crate) fn queue_wallet_tracker_order_refresh(&mut self, address: String) {
        let tracker = &mut self.wallet_tracker;
        if tracker.tracked_addresses.contains(&address)
            && !tracker
                .rows
                .get(&address)
                .is_some_and(|row| row.order_loading)
            && !tracker.order_refresh_queue.contains(&address)
        {
            tracker.order_refresh_queue.push(address);
        }
    }
}
