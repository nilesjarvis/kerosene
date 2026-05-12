use crate::app_state::TradingTerminal;
use crate::message::Message;

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
                self.order_status = Some((
                    format!(
                        "Cannot NUKE: {} position{} unresolvable: {}",
                        plan.skipped.len(),
                        if plan.skipped.len() == 1 { "" } else { "s" },
                        plan.format_skip_list()
                    ),
                    true,
                ));
                return Task::none();
            }

            self.nuke_confirmation = Some(now);
            let ready_count = plan.ready.len();
            let ready_list = plan.format_ready_list();
            let message = if plan.skipped.is_empty() {
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
            };
            self.order_status = Some((message, true));
            return Task::none();
        }
        self.nuke_confirmation = None;
        self.execute_nuke_positions()
    }
}

pub(crate) fn nuke_confirmation_is_armed(armed_at: Option<Instant>, now: Instant) -> bool {
    armed_at.is_some_and(|armed_at| now.duration_since(armed_at) <= NUKE_CONFIRMATION_WINDOW)
}

#[cfg(test)]
mod tests {
    use super::{NUKE_CONFIRMATION_WINDOW, nuke_confirmation_is_armed};
    use std::time::{Duration, Instant};

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
