use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::reject_if_positions_incomplete_for_action;

use iced::Task;
use std::time::Instant;

mod confirmation;

use confirmation::nuke_arm_status_for_plan;
pub(crate) use confirmation::{NukeConfirmation, nuke_confirmation_is_armed};

impl TradingTerminal {
    pub(crate) fn handle_nuke_positions(&mut self) -> Task<Message> {
        let now = Instant::now();
        let armed = nuke_confirmation_is_armed(self.nuke_confirmation.as_ref(), now);
        if self.reject_if_pending_trading_request("NUKE") {
            self.nuke_confirmation = None;
            return Task::none();
        }
        if let Some(task) = self.nuke_position_action_preflight_task() {
            self.nuke_confirmation = None;
            return task;
        }
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

            let task = self.arm_nuke_confirmation_for_plan(now, plan);
            if self.nuke_confirmation.is_some() {
                self.close_menu_coin = None;
            }
            return task;
        }

        let plan = match self.plan_nuke_positions() {
            Ok(plan) => plan,
            Err(e) => {
                self.nuke_confirmation = None;
                self.order_status = Some((e, true));
                return Task::none();
            }
        };
        let account_address = self.connected_order_account_address();
        if !self.nuke_confirmation.as_ref().is_some_and(|confirmation| {
            confirmation.matches_plan(account_address.as_deref(), &plan)
        }) {
            let task = self.arm_nuke_confirmation_for_plan(now, plan);
            if self.nuke_confirmation.is_some() {
                self.close_menu_coin = None;
            }
            return task;
        }
        self.close_menu_coin = None;
        self.nuke_confirmation = None;
        self.execute_nuke_positions()
    }

    fn nuke_position_action_preflight_task(&mut self) -> Option<Task<Message>> {
        let Some((_, account_address)) = self.order_signing_context() else {
            return Some(Task::none());
        };
        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh account data before NUKE".into(),
                true,
            ));
            return Some(Task::none());
        }
        if self.reject_if_account_reconciliation_required("NUKE", "account data") {
            return Some(Task::none());
        }
        if let Some(task) = reject_if_positions_incomplete_for_action(self, "NUKE") {
            return Some(task);
        }
        let Some(account_data) = self.account_data_for_order_account(&account_address) else {
            self.order_status = Some((
                "No account data available; refresh before NUKE".into(),
                true,
            ));
            return Some(Task::none());
        };
        let now_ms = Self::now_ms();
        if !account_data.is_fresh_for_position_action(now_ms) {
            let age_label = account_data
                .position_action_snapshot_age_ms(now_ms)
                .map(|age| format!("{}s old", age.div_ceil(1000)))
                .unwrap_or_else(|| "from the future".to_string());
            self.order_status = Some((
                format!("Account data is stale ({age_label}); refresh before NUKE"),
                true,
            ));
            return Some(self.refresh_account_data());
        }

        None
    }

    fn arm_nuke_confirmation_for_plan(
        &mut self,
        armed_at: Instant,
        plan: crate::order_execution::NukePlan,
    ) -> Task<Message> {
        self.nuke_confirmation = None;
        if plan.is_empty() {
            self.order_status = Some(("No positions to close".into(), true));
            return Task::none();
        }
        if !plan.hidden_skipped.is_empty() {
            self.order_status = Some((nuke_arm_status_for_plan(&plan), true));
            return Task::none();
        }
        if plan.ready.is_empty() {
            // Nothing routable — refuse to arm so the user sees the problem
            // (degraded mid feed, missing symbol metadata, ...) rather than
            // confirming a no-op.
            self.order_status = Some((nuke_arm_status_for_plan(&plan), true));
            return Task::none();
        }

        let account_address = self.connected_order_account_address();
        self.nuke_confirmation = Some(NukeConfirmation::new(
            armed_at,
            account_address.as_deref(),
            &plan,
        ));
        self.order_status = Some((nuke_arm_status_for_plan(&plan), true));
        Task::none()
    }
}

#[cfg(test)]
mod tests;
