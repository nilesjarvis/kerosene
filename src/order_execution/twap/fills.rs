use crate::account::UserFill;
use crate::app_state::TradingTerminal;
use crate::signing::float_to_wire;
use crate::twap_state::{TWAP_RECONCILIATION_TIMEOUT, TwapEventKind, TwapOrder, TwapStatus};

use std::time::Instant;

// ---------------------------------------------------------------------------
// TWAP Account Fill Reconciliation
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn reconcile_twap_fills_from_account(&mut self) {
        let Some(data) = self.account_data.as_ref() else {
            return;
        };
        let Some(address) = self.connected_address.clone() else {
            return;
        };
        let fills = data.fills.clone();
        self.reconcile_twap_fills_for_account(&address, &fills);
    }

    pub(crate) fn reconcile_twap_fills_for_account(
        &mut self,
        account_address: &str,
        fills: &[UserFill],
    ) {
        let now = Instant::now();
        let mut archive_ids = Vec::new();
        for twap in self.twap_orders.values_mut() {
            if twap.account_address != account_address {
                continue;
            }
            let before = twap.filled_size;
            let before_status = twap.status;
            twap.reconcile_fills(fills);
            if twap.filled_size > before {
                if before_status == TwapStatus::Paused && !twap.has_status_unknown_child() {
                    twap.status_check_cloid = None;
                    twap.reconciliation_deadline = None;
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        "TWAP resumed after account fill reconciliation".to_string(),
                        false,
                    );
                }
                twap.push_event(
                    TwapEventKind::Filled,
                    format!(
                        "Reconciled fills: {} / {} filled",
                        float_to_wire(twap.filled_size),
                        float_to_wire(twap.target_size)
                    ),
                    false,
                );
            } else if TwapOrder::reconciliation_timed_out(twap.reconciliation_deadline, now)
                && twap.has_status_unknown_child()
            {
                // The exchange reported a child as filled, but `account.fills`
                // never echoed it within TWAP_RECONCILIATION_TIMEOUT. Tear
                // the TWAP down with a clear, actionable error rather than
                // leave it paused indefinitely with `status_check_cloid` set
                // (which would block every future slice via `can_schedule_at`).
                let pending_cloid = twap.status_check_cloid.clone().unwrap_or_default();
                twap.status_check_cloid = None;
                twap.reconciliation_deadline = None;
                twap.status = TwapStatus::Error;
                twap.push_event(
                    TwapEventKind::Error,
                    format!(
                        "Could not reconcile slice {pending_cloid} via account fills within {}s; \
                         exchange reported fill but account fills did not catch up. Check the \
                         exchange before manually resuming.",
                        TWAP_RECONCILIATION_TIMEOUT.as_secs()
                    ),
                    true,
                );
            }
            if twap.status.is_terminal()
                && (twap.filled_size > before || twap.status != before_status)
            {
                archive_ids.push(twap.id);
            }
        }
        for twap_id in archive_ids {
            self.archive_twap_if_terminal(twap_id);
        }
    }
}
