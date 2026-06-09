use crate::order_execution::NukePlan;

use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// NUKE Confirmation
// ---------------------------------------------------------------------------

pub(super) const NUKE_CONFIRMATION_WINDOW: Duration = Duration::from_secs(5);

pub(super) fn nuke_arm_status_for_plan(plan: &NukePlan) -> String {
    if plan.is_empty() {
        return "No positions to close".to_string();
    }
    if !plan.hidden_skipped.is_empty() {
        return format!(
            "Cannot NUKE: hidden exposure unresolvable: {}",
            plan.format_hidden_skip_list()
        );
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
            concat!(
                "NUKE armed: will close {} ({}); SKIPPING {}. ",
                "Press NUKE again within 5 seconds to fire partial nuke."
            ),
            ready_count,
            ready_list,
            plan.format_skip_list()
        )
    }
}

pub(crate) fn nuke_confirmation_is_armed(armed_at: Option<Instant>, now: Instant) -> bool {
    armed_at.is_some_and(|armed_at| now.duration_since(armed_at) <= NUKE_CONFIRMATION_WINDOW)
}
