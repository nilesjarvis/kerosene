use super::*;

#[test]
fn position_action_snapshot_is_fresh_only_within_cutoff() {
    let data = account_data_snapshot(1_000);

    assert!(data.is_fresh_for_position_action(1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS));
    assert!(
        !data.is_fresh_for_position_action(1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS + 1)
    );
    assert!(!data.is_fresh_for_position_action(999));
}
