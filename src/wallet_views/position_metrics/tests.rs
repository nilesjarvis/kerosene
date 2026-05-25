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
fn wallet_position_upnl_prefers_live_mark_only_with_valid_inputs() {
    assert_eq!(
        wallet_position_upnl(Some(2.0), Some(90.0), "1", Some(100.0)),
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
