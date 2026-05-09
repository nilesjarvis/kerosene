use super::*;

#[test]
fn non_perp_coin_detection_covers_spot_and_outcome_keys() {
    assert!(is_non_perp_coin("@107"));
    assert!(is_non_perp_coin("#0"));
    assert!(!is_non_perp_coin("BTC"));
    assert!(!is_non_perp_coin("xyz:NVDA"));
}

#[test]
fn signed_fill_size_preserves_existing_side_mapping() {
    assert_eq!(signed_fill_size("A", 2.5), -2.5);
    assert_eq!(signed_fill_size("B", 2.5), 2.5);
    assert_eq!(signed_fill_size("unknown", 2.5), 2.5);
}

#[test]
fn resolved_start_position_uses_same_timestamp_tracked_position() {
    assert_eq!(resolved_start_position(1.0, Some((10, 2.0)), 10), 2.0);
    assert_eq!(resolved_start_position(1.0, Some((9, 2.0)), 10), 1.0);
    assert_eq!(resolved_start_position(1.0, None, 10), 1.0);
}

#[test]
fn position_transition_detects_close_and_flip() {
    assert_eq!(
        fill_position_transition(1.0, -1.0, false),
        FillPositionTransition {
            new_pos: 0.0,
            is_flip: false,
            is_close: true,
        }
    );
    assert_eq!(
        fill_position_transition(1.0, -2.5, false),
        FillPositionTransition {
            new_pos: -1.5,
            is_flip: true,
            is_close: false,
        }
    );
    assert_eq!(
        fill_position_transition(-1.0, 2.5, false),
        FillPositionTransition {
            new_pos: 1.5,
            is_flip: true,
            is_close: false,
        }
    );
}

#[test]
fn settlement_transition_keeps_position_unchanged() {
    assert_eq!(
        fill_position_transition(1.0, -5.0, true),
        FillPositionTransition {
            new_pos: 1.0,
            is_flip: false,
            is_close: false,
        }
    );
}
