use super::*;

#[test]
fn position_row_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_position_row_number(" 2.5 "), Some(2.5));
    assert_eq!(parse_position_row_number("-0.125"), Some(-0.125));

    assert_eq!(parse_position_row_number("not-a-number"), None);
    assert_eq!(parse_position_row_number("NaN"), None);
    assert_eq!(parse_position_row_number("inf"), None);
}

#[test]
fn position_row_value_prefers_live_mid_only_when_inputs_are_valid() {
    assert_eq!(
        position_value_from(Some(100.0), Some(-2.0), Some(999.0)),
        Some(200.0)
    );
    assert_eq!(
        position_value_from(Some(100.0), None, Some(999.0)),
        Some(999.0)
    );
    assert_eq!(
        position_value_from(None, Some(-2.0), Some(-250.0)),
        Some(250.0)
    );
    assert_eq!(position_value_from(None, Some(-2.0), None), None);
}

#[test]
fn position_row_upnl_prefers_live_mid_only_when_inputs_are_valid() {
    assert_eq!(
        unrealized_pnl_from(Some(100.0), Some(2.0), Some(90.0), Some(1.0)),
        Some(20.0)
    );
    assert_eq!(
        unrealized_pnl_from(Some(100.0), None, Some(90.0), Some(1.0)),
        Some(1.0)
    );
    assert_eq!(unrealized_pnl_from(None, Some(2.0), Some(90.0), None), None);
}
