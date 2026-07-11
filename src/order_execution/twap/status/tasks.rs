use super::TwapAccountRefresh;
use crate::api::fetch_order_status_by_cloid;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;
use std::time::Duration;

// ---------------------------------------------------------------------------
// TWAP Status Tasks
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::order_execution::twap) fn refresh_after_twap_result(
        &mut self,
        policy: TwapAccountRefresh,
        twap_id: u64,
    ) -> Task<Message> {
        match policy {
            TwapAccountRefresh::Immediate => {
                let Some(addr) = self.twap_origin_address(twap_id) else {
                    return Task::none();
                };
                self.refresh_account_data_for_twap_reconciliation(addr)
            }
            _ if self.twap_refresh_policy_needs_refresh(policy, twap_id) => {
                let Some(addr) = self.twap_origin_address(twap_id) else {
                    return Task::none();
                };
                self.refresh_account_data_for_twap_reconciliation(addr)
            }
            _ => Task::none(),
        }
    }

    pub(in crate::order_execution::twap) fn twap_origin_address(
        &self,
        twap_id: u64,
    ) -> Option<String> {
        self.twap_orders
            .get(&twap_id)
            .map(|twap| twap.account_address.clone())
    }

    pub(in crate::order_execution::twap) fn check_twap_child_status(
        &mut self,
        twap_id: u64,
        cloid: String,
    ) -> Task<Message> {
        let Some(address) = self.twap_origin_address(twap_id) else {
            return Task::none();
        };
        let request_cloid = cloid.clone();
        Task::perform(
            fetch_order_status_by_cloid(address, request_cloid),
            move |result| Message::TwapOrderStatusLoaded {
                twap_id,
                cloid: cloid.clone().into(),
                result: Box::new(result),
            },
        )
    }

    pub(in crate::order_execution::twap) fn check_twap_child_status_after(
        &mut self,
        twap_id: u64,
        cloid: String,
        delay: Duration,
    ) -> Task<Message> {
        let Some(address) = self.twap_origin_address(twap_id) else {
            return Task::none();
        };
        let request_cloid = cloid.clone();
        Task::perform(
            async move {
                tokio::time::sleep(delay).await;
                fetch_order_status_by_cloid(address, request_cloid).await
            },
            move |result| Message::TwapOrderStatusLoaded {
                twap_id,
                cloid: cloid.clone().into(),
                result: Box::new(result),
            },
        )
    }

    pub(in crate::order_execution::twap) fn twap_refresh_policy_needs_refresh(
        &self,
        policy: TwapAccountRefresh,
        twap_id: u64,
    ) -> bool {
        let twap_is_terminal = self
            .twap_orders
            .get(&twap_id)
            .is_some_and(|twap| twap.status.is_terminal());
        policy.should_refresh(twap_is_terminal)
    }
}
