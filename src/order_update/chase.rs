mod cancel;
mod modify;
mod resting;
mod result;

use crate::api::fetch_order_status_by_oid;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason};
use iced::Task;
use std::time::Instant;

impl TradingTerminal {
    pub(crate) fn check_chase_order_status(
        &mut self,
        chase_id: u64,
        oid: u64,
        status: impl Into<String>,
    ) -> Task<Message> {
        let status = status.into();
        let can_refresh_chase_account = self.chase_orders.get(&chase_id).is_some_and(|chase| {
            self.connected_address.as_deref() == Some(chase.account_address.as_str())
        });
        let account_address = self
            .chase_orders
            .get(&chase_id)
            .map(|chase| chase.account_address.clone());
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.current_oid = Some(oid);
            chase.lifecycle = if chase.lifecycle.is_stopping() {
                ChaseLifecycle::Stopping {
                    phase: ChaseStopPhase::VerifyingCancel { oid },
                }
            } else {
                ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Modify,
                }
            };
            chase.last_reprice_at = Some(Instant::now());
        }
        if can_refresh_chase_account {
            self.order_status = Some((status, false));
            let status_task = account_address.map_or_else(Task::none, |account_address| {
                Task::perform(fetch_order_status_by_oid(account_address, oid), move |result| {
                    Message::ChaseOrderOidStatusLoaded {
                        chase_id,
                        oid,
                        result: Box::new(result),
                    }
                })
            });
            Task::batch([self.refresh_account_data(), status_task])
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
