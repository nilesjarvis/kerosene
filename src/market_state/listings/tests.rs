use super::*;

fn market(id: &str, active: bool) -> ListingMarket {
    ListingMarket {
        id: id.into(),
        key: id.into(),
        label: id.into(),
        active,
    }
}

fn snapshot(perps: Vec<ListingMarket>, spot: Vec<ListingMarket>) -> ListingsSnapshot {
    ListingsSnapshot {
        perps: Ok(perps),
        spot: Ok(spot),
    }
}

#[test]
fn listings_baseline_is_silent_and_new_markets_are_deduplicated() {
    let mut state = ListingsState::default();
    assert!(!state.apply(
        snapshot(vec![market("BTC", true)], vec![market("spot:0", true)]),
        100
    ));
    let next = snapshot(
        vec![market("BTC", true), market("xyz:NEW", true)],
        vec![market("spot:0", true), market("spot:107", true)],
    );
    assert!(state.apply(next.clone(), 200));
    assert!(!state.apply(next, 300));
    assert_eq!(state.history.events.len(), 2);
    assert!(
        state
            .history
            .events
            .iter()
            .all(|event| event.detected_at_ms == 200)
    );
    assert_eq!(
        state
            .history
            .events
            .iter()
            .filter(|event| event.kind == ListingKind::Spot)
            .count(),
        1
    );
}

#[test]
fn listings_partial_failure_and_recovery_do_not_reset_baselines() {
    let mut state = ListingsState::default();
    state.apply(
        ListingsSnapshot {
            perps: Err("offline".into()),
            spot: Ok(vec![market("spot:0", true)]),
        },
        100,
    );
    assert!(!state.history.perps.initialized);
    assert!(state.history.spot.initialized);
    state.apply(
        snapshot(
            vec![market("BTC", true)],
            vec![market("spot:0", true), market("spot:1", true)],
        ),
        200,
    );
    assert_eq!(state.history.events.len(), 1);
    assert_eq!(state.history.events[0].id, "spot:1");
    state.apply(
        ListingsSnapshot {
            perps: Err("offline".into()),
            spot: Err("offline".into()),
        },
        300,
    );
    state.apply(
        snapshot(
            vec![market("BTC", true)],
            vec![market("spot:0", true), market("spot:1", true)],
        ),
        400,
    );
    assert_eq!(state.history.events.len(), 1);
    assert!(state.error.is_none());
}

#[test]
fn listings_inactive_baseline_and_reappearing_markets_are_not_new() {
    let mut state = ListingsState::default();
    state.apply(
        snapshot(
            vec![market("OLD", false), market("BTC", true)],
            vec![market("spot:0", true)],
        ),
        100,
    );
    state.apply(
        snapshot(vec![market("BTC", true)], vec![market("spot:0", true)]),
        200,
    );
    assert!(!state.apply(
        snapshot(
            vec![market("OLD", true), market("BTC", true)],
            vec![market("spot:0", true)]
        ),
        300
    ));
    assert!(state.history.events.is_empty());
}

#[test]
fn listings_new_inactive_market_is_announced_only_on_first_activation() {
    let mut state = ListingsState::default();
    state.apply(
        snapshot(vec![market("BTC", true)], vec![market("spot:0", true)]),
        100,
    );
    state.apply(
        snapshot(
            vec![market("BTC", true), market("xyz:NEW", false)],
            vec![market("spot:0", true)],
        ),
        200,
    );
    assert!(state.history.events.is_empty());
    let active = snapshot(
        vec![market("BTC", true), market("xyz:NEW", true)],
        vec![market("spot:0", true)],
    );
    assert!(state.apply(active.clone(), 300));
    assert!(!state.apply(active, 400));
    assert_eq!(state.history.events[0].detected_at_ms, 300);
}

#[test]
fn listings_stable_pair_id_prevents_alias_or_label_change_alerts() {
    let mut state = ListingsState::default();
    state.apply(
        snapshot(vec![market("BTC", true)], vec![market("spot:10000", true)]),
        100,
    );
    let mut renamed = market("spot:10000", true);
    renamed.key = "PURR/USDC".into();
    renamed.label = "Purr / USDC".into();
    assert!(!state.apply(snapshot(vec![market("BTC", true)], vec![renamed]), 200));
}

#[test]
fn listings_history_is_bounded_but_old_identities_remain_known() {
    let mut state = ListingsState::default();
    state.apply(
        snapshot(vec![market("BTC", true)], vec![market("spot:0", true)]),
        100,
    );
    for index in 0..MAX_LISTING_EVENTS + 10 {
        state.apply(
            snapshot(
                vec![market(&format!("NEW{index}"), true)],
                vec![market("spot:0", true)],
            ),
            200 + index as u64,
        );
    }
    assert_eq!(state.history.events.len(), MAX_LISTING_EVENTS);
    assert_eq!(
        state.history.events[0].id,
        format!("NEW{}", MAX_LISTING_EVENTS + 9)
    );
    assert!(!state.apply(
        snapshot(vec![market("NEW0", true)], vec![market("spot:0", true)]),
        999
    ));
}

#[test]
fn listings_restart_preserves_baseline_and_catches_offline_additions() {
    let mut state = ListingsState::default();
    state.apply(
        snapshot(vec![market("BTC", true)], vec![market("spot:0", true)]),
        100,
    );
    let bytes = serde_json::to_vec(&state.history).expect("history serializes");
    let mut restored = ListingsState {
        history: serde_json::from_slice(&bytes).expect("history restores"),
        ..Default::default()
    };
    assert!(restored.apply(
        snapshot(
            vec![market("BTC", true), market("NEW", true)],
            vec![market("spot:0", true)]
        ),
        500
    ));
    assert_eq!(restored.history.events[0].detected_at_ms, 500);
}

#[test]
fn listings_empty_snapshots_do_not_seed_baseline() {
    let mut state = ListingsState::default();
    state.apply(snapshot(Vec::new(), Vec::new()), 100);
    assert!(!state.history.perps.initialized && !state.history.spot.initialized);
}
