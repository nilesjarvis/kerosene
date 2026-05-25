use super::*;

#[test]
fn account_balance_helpers_use_live_position_and_spot_values() {
    assert_eq!(
        position_summary_position_upnl_value("2", "100", "1", Some(110.0)),
        Some(20.0)
    );
    assert_eq!(
        position_summary_position_upnl_value("bad", "100", "1", Some(110.0)),
        Some(1.0)
    );

    assert_eq!(
        position_summary_spot_balance_value("USDC", "10", "0", None),
        Some(10.0)
    );
    assert_eq!(
        position_summary_spot_balance_value("PURR", "2", "3", Some(4.0)),
        Some(8.0)
    );
    assert_eq!(
        position_summary_spot_balance_value("PURR", "2", "3", None),
        Some(3.0)
    );
}
