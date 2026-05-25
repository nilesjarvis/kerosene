use super::confirmation::{
    NUKE_CONFIRMATION_WINDOW, nuke_arm_status_for_plan, nuke_confirmation_is_armed,
};
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
        concat!(
            "NUKE armed: will close 1 (BTC); SKIPPING SHIB (no mid price). ",
            "Press NUKE again within 5 seconds to fire partial nuke."
        )
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
