use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;
use std::time::Instant;

mod confirmation;

use confirmation::nuke_arm_status_for_plan;
pub(crate) use confirmation::nuke_confirmation_is_armed;

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
            if !plan.hidden_skipped.is_empty() {
                self.order_status = Some((nuke_arm_status_for_plan(&plan), true));
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

#[cfg(test)]
mod tests;
