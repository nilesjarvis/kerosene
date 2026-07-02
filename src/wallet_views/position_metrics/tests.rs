use super::*;

#[test]
fn wallet_position_value_prefers_live_mark_only_with_valid_size() {
    assert_eq!(
        wallet_position_value(Some(-2.0), "1", Some(100.0)),
        Some(200.0)
    );
    assert_eq!(wallet_position_value(None, "3", Some(100.0)), Some(3.0));
    assert_eq!(wallet_position_value(None, "bad", Some(100.0)), None);
}

#[test]
fn wallet_spot_value_unavailable_requires_spot_row_with_empty_wire() {
    // Basis-less synthesized spot rows: empty wire string, nothing parsed.
    assert!(wallet_spot_value_unavailable(true, None, ""));
    assert!(wallet_spot_value_unavailable(true, None, "  "));
    // A parsed value (e.g. mark-derived) is available even with empty wire.
    assert!(!wallet_spot_value_unavailable(true, Some(5.0), ""));
    // Non-empty garbage on a spot row is invalid, not unavailable.
    assert!(!wallet_spot_value_unavailable(true, None, "bad"));
    // Perp rows always report missing values as invalid.
    assert!(!wallet_spot_value_unavailable(false, None, ""));
}

#[test]
fn wallet_position_upnl_prefers_live_mark_only_with_valid_inputs() {
    assert_eq!(
        wallet_position_upnl(Some(2.0), Some(90.0), "1", Some(100.0)),
        Some(20.0)
    );
    assert_eq!(
        wallet_position_upnl(Some(-2.0), Some(100.0), "-99", Some(90.0)),
        Some(20.0)
    );
    assert_eq!(
        wallet_position_upnl(Some(2.0), None, "1", Some(100.0)),
        Some(1.0)
    );
    assert_eq!(
        wallet_position_upnl(Some(2.0), None, "bad", Some(100.0)),
        None
    );
}
