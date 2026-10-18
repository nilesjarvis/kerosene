use super::*;
use crate::config::updates::{MarginTier, MarketMarginMode};

fn decimal(value: &str) -> ParameterDecimal {
    ParameterDecimal::try_from(value.to_string()).expect("decimal")
}

fn snapshot(leverage: u32, cap: &str) -> UpdatesSnapshot {
    UpdatesSnapshot {
        parameters: Ok(BTreeMap::from([(
            "xyz:BTC".into(),
            AssetParameters {
                max_leverage: leverage,
                margin_tiers: vec![MarginTier {
                    lower_bound: decimal("0"),
                    max_leverage: leverage,
                }],
                margin_mode: Some(MarketMarginMode::Normal),
            },
        )])),
        caps: Ok(BTreeMap::from([("xyz:BTC".into(), decimal(cap))])),
    }
}

#[test]
fn updates_seed_silently_group_changes_and_deduplicate_identical_snapshots() {
    let mut state = UpdatesState::default();
    state.apply(snapshot(20, "100.00"), 1_000);
    assert!(state.history.events.is_empty());
    state.apply(snapshot(10, "150"), 31_000);
    assert_eq!(state.history.events.len(), 1);
    let event = &state.history.events[0];
    assert_eq!(event.key, "xyz:BTC");
    assert_eq!(
        event.changes.len(),
        2,
        "flat leverage change is not repeated as a tier change"
    );
    assert_eq!(event.previous_observed_at_ms, 1_000);
    assert!(!event.after_gap);
    state.apply(snapshot(10, "0150.0"), 61_000);
    assert_eq!(state.history.events.len(), 1);
}

#[test]
fn updates_single_tier_change_without_max_leverage_change_is_reported() {
    let mut state = UpdatesState::default();
    state.apply(snapshot(20, "100"), 1_000);
    let mut next = snapshot(20, "100");
    next.parameters
        .as_mut()
        .expect("parameters")
        .get_mut("xyz:BTC")
        .expect("asset")
        .margin_tiers[0]
        .max_leverage = 10;
    state.apply(next, 31_000);
    assert!(matches!(
        state.history.events[0].changes.as_slice(),
        [ParameterChange::MarginTiers { .. }]
    ));
}

#[test]
fn updates_failed_families_keep_baseline_and_recovery_marks_gap() {
    let mut state = UpdatesState::default();
    state.apply(snapshot(20, "100"), 1_000);
    let mut partial = snapshot(10, "200");
    partial.caps = Err("offline".into());
    state.apply(partial, 31_000);
    assert_eq!(state.history.caps["xyz:BTC"], decimal("100"));
    assert_eq!(state.history.caps_observed_at_ms, Some(1_000));
    assert_eq!(state.history.events.len(), 1);
    state.apply(snapshot(10, "200"), 61_000);
    assert_eq!(state.history.events.len(), 2);
    assert!(state.history.events[0].after_gap);
    assert_eq!(state.history.events[0].previous_observed_at_ms, 1_000);
    assert_eq!(state.failures, 0);
    assert!(state.error.is_none());
}

#[test]
fn updates_restart_and_long_poll_gaps_preserve_observation_intervals() {
    let mut state = UpdatesState::default();
    state.apply(snapshot(20, "100"), 1_000);
    let mut restored = UpdatesState {
        history: state.history,
        ..UpdatesState::default()
    };
    restored.apply(snapshot(10, "200"), 31_000);
    assert!(
        restored.history.events[0].after_gap,
        "restart is a monitoring gap even if short"
    );
    restored.apply(snapshot(5, "300"), 200_000);
    assert!(restored.history.events[0].after_gap);
}

#[test]
fn updates_unpublished_caps_and_new_assets_never_become_zero_or_unlimited_changes() {
    let mut state = UpdatesState::default();
    state.apply(snapshot(20, "100"), 1_000);
    let mut missing = snapshot(20, "100");
    missing.caps = Ok(BTreeMap::new());
    state.apply(missing, 31_000);
    assert!(state.history.events.is_empty());
    state.apply(snapshot(20, "200"), 61_000);
    assert!(
        state.history.events.is_empty(),
        "republished cap seeds silently"
    );
    let mut added = snapshot(20, "200");
    added
        .caps
        .as_mut()
        .expect("caps")
        .insert("flx:BTC".into(), decimal("500"));
    state.apply(added, 91_000);
    assert!(state.history.events.is_empty());
}

#[test]
fn updates_compare_tier_contents_and_known_margin_modes() {
    let mut state = UpdatesState::default();
    let mut before = snapshot(20, "100");
    let parameters = before
        .parameters
        .as_mut()
        .expect("params")
        .get_mut("xyz:BTC")
        .expect("asset");
    parameters.margin_tiers.push(MarginTier {
        lower_bound: decimal("1000000"),
        max_leverage: 10,
    });
    state.apply(before.clone(), 1_000);
    let mut after = before.clone();
    let parameters = after
        .parameters
        .as_mut()
        .expect("params")
        .get_mut("xyz:BTC")
        .expect("asset");
    parameters.margin_tiers[1].lower_bound = decimal("2000000");
    parameters.margin_mode = Some(MarketMarginMode::NoCross);
    state.apply(after, 31_000);
    let changes = &state.history.events[0].changes;
    assert!(
        changes
            .iter()
            .any(|change| matches!(change, ParameterChange::MarginTiers { .. }))
    );
    assert!(
        changes
            .iter()
            .any(|change| matches!(change, ParameterChange::MarginMode { .. }))
    );
    let parameters = before
        .parameters
        .as_mut()
        .expect("params")
        .get_mut("xyz:BTC")
        .expect("asset");
    parameters.margin_mode = None;
    state.apply(before, 61_000);
    assert!(
        !state.history.events[0]
            .changes
            .iter()
            .any(|change| matches!(change, ParameterChange::MarginMode { .. }))
    );
}

#[test]
fn updates_history_and_retry_delay_are_bounded_and_filters_respect_dex_keys() {
    let mut state = UpdatesState::default();
    for n in 0..MAX_UPDATE_EVENTS + 10 {
        state.apply(snapshot(20, &n.to_string()), n as u64 * 30_000);
    }
    assert_eq!(state.history.events.len(), MAX_UPDATE_EVENTS);
    let event = state.history.events[0].clone();
    state.venue = UpdatesVenue::Dex("flx".into());
    assert!(!state.matches(&event, true));
    state.venue = UpdatesVenue::Dex("xyz".into());
    state.favourites_only = true;
    assert!(!state.matches(&event, false));
    assert!(state.matches(&event, true));
    state.search = "bTc".into();
    assert!(state.matches(&event, true));
    state.filter = UpdatesFilter::Kind(ParameterKind::MarginMode);
    assert!(!state.matches(&event, true));
    for _ in 0..20 {
        state.apply(
            UpdatesSnapshot {
                parameters: Err("offline".into()),
                caps: Err("offline".into()),
            },
            1_000_000,
        );
    }
    assert_eq!(state.refresh_secs(), 300);
}
