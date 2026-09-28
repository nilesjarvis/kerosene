mod cancel;
mod modify;
mod resting;
mod result;

pub(super) use cancel::chase_terminal_cancel_error;

use crate::api::fetch_order_status_by_oid;
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use iced::Task;
use std::time::Instant;

impl TradingTerminal {
    pub(super) fn archive_disconnected_stopping_chase(
        &mut self,
        chase_id: u64,
        summary: String,
    ) -> bool {
        let should_archive = self.chase_orders.get(&chase_id).is_some_and(|chase| {
            chase.lifecycle.is_stopping()
                && !self.connected_order_account_matches(&chase.account_address)
        });
        if should_archive {
            self.remove_chase_order_with_summary(chase_id, Some(summary));
        }
        should_archive
    }

    pub(super) fn refresh_account_data_for_order_account(
        &mut self,
        account_address: &str,
    ) -> Task<Message> {
        if self.connected_order_account_matches(account_address) {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }

    pub(crate) fn check_chase_order_status(
        &mut self,
        chase_id: u64,
        oid: u64,
        status: impl Into<String>,
    ) -> Task<Message> {
        let status = redact_sensitive_response_text(&status.into());
        let account_address = self.chase_orders.get_mut(&chase_id).map(|chase| {
            let account_address = chase.account_address.clone();
            chase.current_oid = Some(oid);
            chase.lifecycle = chase.lifecycle.verifying_order_status(oid);
            chase.last_reprice_at = Some(Instant::now());
            account_address
        });
        let can_refresh_chase_account = account_address
            .as_deref()
            .is_some_and(|address| self.connected_order_account_matches(address));
        let status_task = account_address.map_or_else(Task::none, |account_address| {
            Task::perform(
                fetch_order_status_by_oid(account_address, oid),
                move |result| Message::ChaseOrderOidStatusLoaded {
                    chase_id,
                    oid,
                    result: Box::new(result),
                },
            )
        });
        if can_refresh_chase_account {
            self.order_status = Some((status, false));
            Task::batch([self.refresh_account_data(), status_task])
        } else {
            self.set_order_status(
                format!(
                    "{status}; checking previous account order status without clearing chase state"
                ),
                true,
            );
            status_task
        }
    }
}
