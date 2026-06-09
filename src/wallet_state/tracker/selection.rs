use crate::app_state::TradingTerminal;
use crate::wallet_state::model::{WALLET_TRACKER_CORE_MIN_AGE_MS, WALLET_TRACKER_ORDER_MIN_AGE_MS};

impl TradingTerminal {
    pub(crate) fn wallet_tracker_next_core_address(&mut self, now_ms: u64) -> Option<String> {
        self.wallet_tracker_next_core_addresses(now_ms, 1)
            .into_iter()
            .next()
    }

    pub(crate) fn wallet_tracker_next_core_addresses(
        &mut self,
        now_ms: u64,
        max_count: usize,
    ) -> Vec<String> {
        if self.wallet_tracker.rows.values().any(|row| row.loading) {
            return Vec::new();
        }

        let max_count = max_count.max(1);
        let mut addresses = Vec::new();
        while !self.wallet_tracker.core_refresh_queue.is_empty() {
            let address = self.wallet_tracker.core_refresh_queue.remove(0);
            let Some(row) = self.wallet_tracker.rows.get(&address) else {
                continue;
            };
            if !self.wallet_tracker.tracked_addresses.contains(&address)
                || row.loading
                || row.next_core_retry_ms.is_some_and(|retry| now_ms < retry)
            {
                continue;
            }
            addresses.push(address);
            if addresses.len() >= max_count {
                return addresses;
            }
        }
        if !addresses.is_empty() {
            return addresses;
        }

        let mut selected = Vec::new();
        for address in &self.wallet_tracker.tracked_addresses {
            let row = self.wallet_tracker.rows.get(address);
            if row.is_some_and(|row| {
                row.loading || row.next_core_retry_ms.is_some_and(|retry| now_ms < retry)
            }) {
                continue;
            }

            let last_updated = row.and_then(|row| row.last_updated_ms).unwrap_or(0);
            let needs_initial_load = row.and_then(|row| row.snapshot.as_ref()).is_none();
            let is_stale = now_ms.saturating_sub(last_updated) >= WALLET_TRACKER_CORE_MIN_AGE_MS;
            if !needs_initial_load && !is_stale {
                continue;
            }

            selected.push((last_updated, address.clone()));
        }
        selected.sort_by_key(|(last_updated, _)| *last_updated);
        selected
            .into_iter()
            .take(max_count)
            .map(|(_, address)| address)
            .collect()
    }

    pub(crate) fn wallet_tracker_next_order_address(&mut self, now_ms: u64) -> Option<String> {
        if self
            .wallet_tracker
            .rows
            .values()
            .any(|row| row.order_loading)
        {
            return None;
        }

        while !self.wallet_tracker.order_refresh_queue.is_empty() {
            let address = self.wallet_tracker.order_refresh_queue.remove(0);
            let Some(row) = self.wallet_tracker.rows.get(&address) else {
                continue;
            };
            if !self.wallet_tracker.tracked_addresses.contains(&address)
                || row.order_loading
                || row.next_order_retry_ms.is_some_and(|retry| now_ms < retry)
            {
                continue;
            }
            return Some(address);
        }

        let mut selected: Option<(u64, String)> = None;
        for address in &self.wallet_tracker.tracked_addresses {
            let Some(row) = self.wallet_tracker.rows.get(address) else {
                continue;
            };
            if row.snapshot.is_none()
                || row.order_loading
                || row.next_order_retry_ms.is_some_and(|retry| now_ms < retry)
            {
                continue;
            }

            let last_updated = row.orders_last_updated_ms.unwrap_or(0);
            let needs_initial_load = row.open_order_count.is_none();
            let is_stale = now_ms.saturating_sub(last_updated) >= WALLET_TRACKER_ORDER_MIN_AGE_MS;
            if !needs_initial_load && !is_stale {
                continue;
            }

            if selected
                .as_ref()
                .is_none_or(|(selected_at, _)| last_updated < *selected_at)
            {
                selected = Some((last_updated, address.clone()));
            }
        }
        selected.map(|(_, address)| address)
    }
}
