use super::*;

#[test]
fn summary_formatting_masks_only_present_values_when_pnl_is_hidden() {
    assert_eq!(
        format_optional_signed_usd(None, true, PositionNumberMode::Full),
        "--"
    );
    assert_eq!(
        format_optional_signed_usd(Some(-12.34), true, PositionNumberMode::Full),
        "$***"
    );
    assert_eq!(
        format_optional_signed_usd(Some(-12.34), false, PositionNumberMode::Full),
        "-$12.34"
    );
}

#[test]
fn total_pnl_display_includes_percent_and_masks_when_hidden() {
    let total = Some(12.5);

    assert_eq!(
        format_optional_total_pnl(total, Some(1.25), false, PositionNumberMode::Full),
        "+$12.50 (+1.25%)"
    );
    assert_eq!(
        format_optional_total_pnl(total, None, false, PositionNumberMode::Full),
        "+$12.50 (--%)"
    );
    assert_eq!(
        format_optional_total_pnl(total, Some(1.25), true, PositionNumberMode::Full),
        "$*** (+1.25%)"
    );
    assert_eq!(
        format_optional_total_pnl(total, None, true, PositionNumberMode::Full),
        "$*** (--%)"
    );
}

#[test]
fn compact_summary_formatting_rounds_money_and_percent() {
    let total = Some(1234.56);

    assert_eq!(
        format_optional_unsigned_usd(total, false, PositionNumberMode::Compact),
        "$1,235"
    );
    assert_eq!(
        format_optional_signed_usd(total, false, PositionNumberMode::Compact),
        "+$1,235"
    );
    assert_eq!(
        format_optional_total_pnl(total, Some(1.25), false, PositionNumberMode::Compact),
        "+$1,235 (+1.2%)"
    );

    let large_total = Some(532_023.0);

    assert_eq!(
        format_optional_unsigned_usd(large_total, false, PositionNumberMode::Compact),
        "$500k"
    );
    assert_eq!(
        format_optional_total_pnl(large_total, Some(1.25), false, PositionNumberMode::Compact),
        "+$500k (+1.2%)"
    );
}
