use super::*;

#[test]
fn wallet_margin_pct_rejects_invalid_or_ambiguous_inputs() {
    assert_eq!(wallet_margin_pct(Some(100.0), Some(25.0)), Some(25.0));
    assert_eq!(wallet_margin_pct(Some(0.0), Some(0.0)), Some(0.0));
    assert_eq!(wallet_margin_pct(Some(0.0), Some(1.0)), None);
    assert_eq!(wallet_margin_pct(None, Some(1.0)), None);
    assert_eq!(wallet_margin_pct(Some(100.0), None), None);
}

#[test]
fn wallet_summary_position_value_prefers_live_mark_only_with_valid_size() {
    assert_eq!(
        wallet_position_value(Some(-2.0), "1", Some(100.0)),
        Some(200.0)
    );
    assert_eq!(wallet_position_value(None, "3", Some(100.0)), Some(3.0));
    assert_eq!(wallet_position_value(None, "bad", Some(100.0)), None);
}

#[test]
fn wallet_summary_position_upnl_prefers_live_mark_only_with_valid_inputs() {
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

#[test]
fn add_optional_poisoned_totals_remain_unknown() {
    let mut total = Some(1.0);
    add_optional(&mut total, Some(2.0));
    assert_eq!(total, Some(3.0));

    add_optional(&mut total, None);
    assert_eq!(total, None);

    add_optional(&mut total, Some(4.0));
    assert_eq!(total, None);
}
