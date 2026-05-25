use crate::app_state::TradingTerminal;
use crate::twap_state::ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL;
use std::time::Instant;

mod limits;
mod place;
mod reprice;
mod stop;
#[cfg(test)]
mod tests;

use limits::*;
#[cfg(test)]
use stop::{StopChaseAction, plan_stop_chase};

impl TradingTerminal {
    pub(crate) fn next_chase_id(&mut self) -> u64 {
        let id = self.next_chase_id;
        self.next_chase_id = self.next_chase_id.checked_add(1).unwrap_or(1);
        id
    }

    fn can_send_chase_exchange_request(&self, now: Instant) -> bool {
        !self.account_loading
            && !self.account_reconciliation_required
            && self.last_advanced_exchange_request_at.is_none_or(|last| {
                now.saturating_duration_since(last) >= ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL
            })
    }
}
