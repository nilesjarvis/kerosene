use super::confirmation::{
    NUKE_CONFIRMATION_WINDOW, NukeConfirmation, nuke_arm_status_for_plan,
    nuke_confirmation_is_armed,
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
        hidden_skipped: vec![],
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
        hidden_skipped: vec![],
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
        hidden_skipped: vec![],
    };

    assert_eq!(
        nuke_arm_status_for_plan(&plan),
        "Cannot NUKE: 2 positions unresolvable: SHIB (no mid price), DOGE (unknown asset)"
    );
}

#[test]
fn nuke_arm_status_refuses_hidden_unrouteable_exposure() {
    let plan = NukePlan {
        ready: vec![("BTC".to_string(), order())],
        skipped: vec![("HIDDEN".to_string(), NukeSkipReason::NoMidPrice)],
        hidden_skipped: vec![("HIDDEN".to_string(), NukeSkipReason::NoMidPrice)],
    };

    assert_eq!(
        nuke_arm_status_for_plan(&plan),
        "Cannot NUKE: hidden exposure unresolvable: HIDDEN (no mid price)"
    );
}

#[test]
fn nuke_confirmation_is_only_armed_inside_window() {
    let now = Instant::now();
    let confirmation = NukeConfirmation::new(
        now - NUKE_CONFIRMATION_WINDOW,
        Some("0xabc"),
        &NukePlan::default(),
    );
    let expired_confirmation = NukeConfirmation::new(
        now - NUKE_CONFIRMATION_WINDOW - Duration::from_millis(1),
        Some("0xabc"),
        &NukePlan::default(),
    );

    assert!(!nuke_confirmation_is_armed(None, now));
    assert!(nuke_confirmation_is_armed(Some(&confirmation), now));
    assert!(!nuke_confirmation_is_armed(
        Some(&expired_confirmation),
        now
    ));
}

#[test]
fn nuke_confirmation_debug_redacts_exact_fingerprint_without_changing_matching() {
    let account_address = "0xabc0000000000000000000000000000000000000";
    let coin = "private-nuke-symbol-sentinel";
    let size = "98765.4321";
    let plan = NukePlan {
        ready: vec![(
            coin.to_string(),
            NukePositionOrder {
                size: size.to_string(),
                ..order()
            },
        )],
        skipped: vec![],
        hidden_skipped: vec![],
    };
    let confirmation = NukeConfirmation::new(Instant::now(), Some(account_address), &plan);

    let rendered = format!("{confirmation:?}");

    assert!(!rendered.contains(account_address));
    assert!(!rendered.contains(coin));
    assert!(!rendered.contains(size));
    assert!(rendered.contains("has_account_address: true"));
    assert!(rendered.contains("ready_count: 1"));
    assert!(confirmation.matches_plan(Some(account_address), &plan));
    assert!(!confirmation.matches_plan(Some("0xdef"), &plan));

    let mut changed_plan = plan.clone();
    changed_plan.ready[0].1.size = "98765.4322".to_string();
    assert!(!confirmation.matches_plan(Some(account_address), &changed_plan));
}
