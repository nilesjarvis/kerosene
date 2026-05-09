use super::*;

#[test]
fn summary_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_summary_number(" 42.5 "), Some(42.5));
    assert_eq!(parse_summary_number("-1.25"), Some(-1.25));

    assert_eq!(parse_summary_number("bad"), None);
    assert_eq!(parse_summary_number("NaN"), None);
    assert_eq!(parse_summary_number("inf"), None);
}

#[test]
fn summary_position_upnl_uses_live_mid_only_with_valid_inputs() {
    assert_eq!(position_upnl_value("2", "90", "1", Some(100.0)), Some(20.0));
    assert_eq!(
        position_upnl_value("bad", "90", "1", Some(100.0)),
        Some(1.0)
    );
    assert_eq!(position_upnl_value("bad", "90", "bad", Some(100.0)), None);
}

#[test]
fn summary_spot_value_does_not_zero_invalid_balances() {
    assert_eq!(spot_balance_value("USDC", "10", "0", None), Some(10.0));
    assert_eq!(spot_balance_value("PURR", "2", "3", Some(4.0)), Some(8.0));
    assert_eq!(spot_balance_value("PURR", "2", "3", None), Some(3.0));
    assert_eq!(spot_balance_value("PURR", "bad", "3", Some(4.0)), None);
    assert_eq!(spot_balance_value("PURR", "2", "bad", None), None);
}

#[test]
fn summary_percent_string_rejects_invalid_margin_ratio() {
    assert_eq!(summary_percent_string(Some(0.125)), "12.50%");
    assert_eq!(summary_percent_string(None), "Invalid data");
}
