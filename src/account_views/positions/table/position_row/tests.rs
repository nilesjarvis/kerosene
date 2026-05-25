use super::super::format_position_usd_value;
use super::*;

#[test]
fn position_entry_price_groups_large_wire_values() {
    assert_eq!(
        format_position_entry_price(Some(12345.678), "12345.678"),
        "12,345.678"
    );
    assert_eq!(
        format_position_entry_price(Some(100000.0), "100000"),
        "100,000"
    );
}

#[test]
fn position_entry_price_preserves_small_wire_values() {
    assert_eq!(
        format_position_entry_price(Some(0.00001234), "0.00001234"),
        "0.00001234"
    );
    assert_eq!(format_position_entry_price(None, "100000"), "Invalid");
}

#[test]
fn compact_position_usd_rounds_to_whole_dollars() {
    assert_eq!(
        format_position_usd_value(1234.56, PositionNumberMode::Full),
        "$1,234.56"
    );
    assert_eq!(
        format_position_usd_value(1234.56, PositionNumberMode::Compact),
        "$1,235"
    );
    assert_eq!(
        format_position_usd_value(-1234.56, PositionNumberMode::Compact),
        "-$1,235"
    );
    assert_eq!(
        format_position_usd_value(0.5, PositionNumberMode::Compact),
        "$1"
    );
    assert_eq!(
        format_position_usd_value(532_023.0, PositionNumberMode::Compact),
        "$500k"
    );
}

#[test]
fn compact_signed_amount_rounds_to_whole_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        format_position_signed_amount(&denomination, 12.34, PositionNumberMode::Full),
        "+$12.34"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, 12345.67, PositionNumberMode::Full),
        "+$12,345.67"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, -1234567.89, PositionNumberMode::Full),
        "-$1,234,567.89"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, 12.56, PositionNumberMode::Compact),
        "+$13"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, -12.56, PositionNumberMode::Compact),
        "-$13"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, 12345.67, PositionNumberMode::Compact),
        "+$12k"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, 532_023.0, PositionNumberMode::Compact),
        "+$500k"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, -1234567.89, PositionNumberMode::Compact),
        "-$1.2M"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, -0.49, PositionNumberMode::Compact),
        "$0"
    );
}

#[test]
fn compact_position_size_trims_unneeded_zeroes() {
    assert_eq!(trim_decimal_zeros(format_size(1.0)), "1");
    assert_eq!(trim_decimal_zeros(format_size(1.25)), "1.25");
    assert_eq!(format_position_compact_number(12_500.0), "13k");
    assert_eq!(format_position_compact_number(532_023.0), "500k");
}
