use crate::account::{
    fetch_wallet_tracker_open_order_count_scoped_with_provider,
    fetch_wallet_tracker_snapshot_scoped_with_provider,
    fetch_wallet_tracker_snapshots_scoped_with_provider, hydromancer_portfolio_chunk_size,
};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn start_wallet_tracker_core_refresh(&mut self, address: String) -> Task<Message> {
        let read_context = self.read_data_request_context();
        self.wallet_tracker
            .rows
            .entry(address.clone())
            .or_default()
            .loading = true;
        if let Some(row) = self.wallet_tracker.rows.get_mut(&address) {
            row.loading_context = Some(read_context);
        }
        let scope = self.account_data_fetch_scope();
        let provider = self.read_data_provider;
        let hydromancer_key = self.hydromancer_api_key_for_task();
        Task::perform(
            fetch_wallet_tracker_snapshot_scoped_with_provider(
                address.clone(),
                scope,
                provider,
                hydromancer_key,
            ),
            move |r| Message::WalletTrackerLoaded(address.clone().into(), read_context, r.into()),
        )
    }

    pub(crate) fn start_wallet_tracker_core_batch_refresh(
        &mut self,
        addresses: Vec<String>,
    ) -> Task<Message> {
        if addresses.is_empty() {
            return Task::none();
        }
        let read_context = self.read_data_request_context();
        for address in &addresses {
            let row = self.wallet_tracker.rows.entry(address.clone()).or_default();
            row.loading = true;
            row.loading_context = Some(read_context);
        }
        let scope = self.account_data_fetch_scope();
        let provider = self.read_data_provider;
        let hydromancer_key = self.hydromancer_api_key_for_task();
        Task::perform(
            fetch_wallet_tracker_snapshots_scoped_with_provider(
                addresses,
                scope,
                provider,
                hydromancer_key,
            ),
            move |r| Message::WalletTrackerBatchLoaded(read_context, r.into()),
        )
    }

    pub(crate) fn start_wallet_tracker_order_refresh(&mut self, address: String) -> Task<Message> {
        let read_context = self.read_data_request_context();
        let row = self.wallet_tracker.rows.entry(address.clone()).or_default();
        row.order_loading = true;
        row.order_loading_context = Some(read_context);
        let scope = self.account_data_fetch_scope();
        let provider = self.read_data_provider;
        let hydromancer_key = self.hydromancer_api_key_for_task();
        Task::perform(
            fetch_wallet_tracker_open_order_count_scoped_with_provider(
                address.clone(),
                scope,
                provider,
                hydromancer_key,
            ),
            move |r| {
                Message::WalletTrackerOrdersLoaded(address.clone().into(), read_context, r.into())
            },
        )
    }

    pub(crate) fn refresh_next_wallet_tracker_core(&mut self) -> Task<Message> {
        let now_ms = Self::now_ms();
        if self.hydromancer_read_provider_enabled() {
            let scope = self.account_data_fetch_scope();
            let addresses = self.wallet_tracker_next_core_addresses(
                now_ms,
                hydromancer_portfolio_chunk_size(&scope),
            );
            return self.start_wallet_tracker_core_batch_refresh(addresses);
        }
        if let Some(address) = self.wallet_tracker_next_core_address(now_ms) {
            return self.start_wallet_tracker_core_refresh(address);
        }
        Task::none()
    }

    pub(crate) fn refresh_next_wallet_tracker_orders(&mut self) -> Task<Message> {
        let now_ms = Self::now_ms();
        if let Some(address) = self.wallet_tracker_next_order_address(now_ms) {
            return self.start_wallet_tracker_order_refresh(address);
        }
        Task::none()
    }
}
