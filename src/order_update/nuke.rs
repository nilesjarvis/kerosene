use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::NukePlan;

use iced::Task;
use std::time::{Duration, Instant};

const NUKE_CONFIRMATION_WINDOW: Duration = Duration::from_secs(5);

impl TradingTerminal {
    pub(crate) fn handle_nuke_positions(&mut self) -> Task<Message> {
        self.close_menu_coin = None;
        let now = Instant::now();
        let armed = nuke_confirmation_is_armed(self.nuke_confirmation, now);
        if !armed {
            // Plan at arm time so the confirmation message surfaces what
            // will actually close (and what won't) — an emergency-flatten
            // control should never let the user confirm in the dark.
            let plan = match self.plan_nuke_positions() {
                Ok(plan) => plan,
                Err(e) => {
                    self.order_status = Some((e, true));
                    return Task::none();
                }
            };

            if plan.is_empty() {
                self.order_status = Some(("No positions to close".into(), true));
                return Task::none();
            }
            if plan.ready.is_empty() {
                // Nothing routable — refuse to arm so the user sees the
                // problem (degraded mid feed, missing symbol metadata, ...)
                // rather than confirming a no-op.
                self.order_status = Some((nuke_arm_status_for_plan(&plan), true));
                return Task::none();
            }

            self.nuke_confirmation = Some(now);
            self.order_status = Some((nuke_arm_status_for_plan(&plan), true));
            return Task::none();
        }
        self.nuke_confirmation = None;
        self.execute_nuke_positions()
    }
}

fn nuke_arm_status_for_plan(plan: &NukePlan) -> String {
    if plan.is_empty() {
        return "No positions to close".to_string();
    }
    if plan.ready.is_empty() {
        return format!(
            "Cannot NUKE: {} position{} unresolvable: {}",
            plan.skipped.len(),
            if plan.skipped.len() == 1 { "" } else { "s" },
            plan.format_skip_list()
        );
    }

    let ready_count = plan.ready.len();
    let ready_list = plan.format_ready_list();
    if plan.skipped.is_empty() {
        format!(
            "NUKE armed: will close {} position{} ({}). Press NUKE again within 5 seconds.",
            ready_count,
            if ready_count == 1 { "" } else { "s" },
            ready_list
        )
    } else {
        format!(
            "NUKE armed: will close {} ({}); SKIPPING {}. Press NUKE again within 5 seconds to fire partial nuke.",
            ready_count,
            ready_list,
            plan.format_skip_list()
        )
    }
}

pub(crate) fn nuke_confirmation_is_armed(armed_at: Option<Instant>, now: Instant) -> bool {
    armed_at.is_some_and(|armed_at| now.duration_since(armed_at) <= NUKE_CONFIRMATION_WINDOW)
}

#[cfg(test)]
mod tests {
    use super::{NUKE_CONFIRMATION_WINDOW, nuke_arm_status_for_plan, nuke_confirmation_is_armed};
    use crate::order_execution::{NukePlan, NukePositionOrder, NukeSkipReason};
    use std::time::{Duration, Instant};

    fn order() -> NukePositionOrder {
        NukePositionOrder {
            asset: 1,
            is_buy: false,
            price: "99".to_string(),
            size: "1".to_string(),
        }
    }

    #[test]
    fn nuke_arm_status_lists_all_ready_positions_when_nothing_is_skipped() {
        let plan = NukePlan {
            ready: vec![("BTC".to_string(), order()), ("ETH".to_string(), order())],
            skipped: vec![],
        };

        assert_eq!(
            nuke_arm_status_for_plan(&plan),
            "NUKE armed: will close 2 positions (BTC, ETH). Press NUKE again within 5 seconds."
        );
    }

    #[test]
    fn nuke_arm_status_warns_before_partial_nuke() {
        let plan = NukePlan {
            ready: vec![("BTC".to_string(), order())],
            skipped: vec![("SHIB".to_string(), NukeSkipReason::NoMidPrice)],
        };

        assert_eq!(
            nuke_arm_status_for_plan(&plan),
            "NUKE armed: will close 1 (BTC); SKIPPING SHIB (no mid price). Press NUKE again within 5 seconds to fire partial nuke."
        );
    }

    #[test]
    fn nuke_arm_status_refuses_all_unrouteable_positions() {
        let plan = NukePlan {
            ready: vec![],
            skipped: vec![
                ("SHIB".to_string(), NukeSkipReason::NoMidPrice),
                ("DOGE".to_string(), NukeSkipReason::UnknownAsset),
            ],
        };

        assert_eq!(
            nuke_arm_status_for_plan(&plan),
            "Cannot NUKE: 2 positions unresolvable: SHIB (no mid price), DOGE (unknown asset)"
        );
    }

    #[test]
    fn nuke_confirmation_is_only_armed_inside_window() {
        let now = Instant::now();

        assert!(!nuke_confirmation_is_armed(None, now));
        assert!(nuke_confirmation_is_armed(
            Some(now - NUKE_CONFIRMATION_WINDOW),
            now
        ));
        assert!(!nuke_confirmation_is_armed(
            Some(now - NUKE_CONFIRMATION_WINDOW - Duration::from_millis(1)),
            now
        ));
    }
}
