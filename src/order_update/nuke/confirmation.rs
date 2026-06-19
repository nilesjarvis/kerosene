use crate::order_execution::NukePlan;

use std::{
    fmt,
    time::{Duration, Instant},
};

// ---------------------------------------------------------------------------
// NUKE Confirmation
// ---------------------------------------------------------------------------

pub(super) const NUKE_CONFIRMATION_WINDOW: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NukeConfirmation {
    armed_at: Instant,
    fingerprint: NukePlanFingerprint,
}

impl NukeConfirmation {
    pub(super) fn new(armed_at: Instant, account_address: Option<&str>, plan: &NukePlan) -> Self {
        Self {
            armed_at,
            fingerprint: NukePlanFingerprint::new(account_address, plan),
        }
    }

    pub(super) fn matches_plan(&self, account_address: Option<&str>, plan: &NukePlan) -> bool {
        self.fingerprint == NukePlanFingerprint::new(account_address, plan)
    }
}

#[derive(Clone, PartialEq, Eq)]
struct NukePlanFingerprint {
    account_address: Option<String>,
    ready: Vec<NukeReadyFingerprint>,
    skipped: Vec<(String, String)>,
    hidden_skipped: Vec<(String, String)>,
}

impl fmt::Debug for NukePlanFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NukePlanFingerprint")
            .field(
                "account_address",
                &self.account_address.as_ref().map(|_| "<redacted>"),
            )
            .field("ready", &self.ready)
            .field("skipped", &self.skipped)
            .field("hidden_skipped", &self.hidden_skipped)
            .finish()
    }
}

impl NukePlanFingerprint {
    fn new(account_address: Option<&str>, plan: &NukePlan) -> Self {
        let mut ready = plan
            .ready
            .iter()
            .map(|(coin, order)| NukeReadyFingerprint {
                coin: coin.clone(),
                asset: order.asset,
                is_buy: order.is_buy,
                size: order.size.clone(),
            })
            .collect::<Vec<_>>();
        ready.sort();

        let mut skipped = plan
            .skipped
            .iter()
            .map(|(coin, reason)| (coin.clone(), format!("{reason:?}")))
            .collect::<Vec<_>>();
        skipped.sort();

        let mut hidden_skipped = plan
            .hidden_skipped
            .iter()
            .map(|(coin, reason)| (coin.clone(), format!("{reason:?}")))
            .collect::<Vec<_>>();
        hidden_skipped.sort();

        Self {
            account_address: account_address.map(str::to_string),
            ready,
            skipped,
            hidden_skipped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NukeReadyFingerprint {
    coin: String,
    asset: u32,
    is_buy: bool,
    size: String,
}

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

pub(crate) fn nuke_confirmation_is_armed(
    confirmation: Option<&NukeConfirmation>,
    now: Instant,
) -> bool {
    confirmation.is_some_and(|confirmation| {
        now.duration_since(confirmation.armed_at) <= NUKE_CONFIRMATION_WINDOW
    })
}
