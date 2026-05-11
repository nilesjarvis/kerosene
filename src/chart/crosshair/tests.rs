use super::format_volume_compact;

#[test]
fn format_volume_compact_handles_zero_and_invalid_inputs() {
    assert_eq!(format_volume_compact(0.0), "0");
    assert_eq!(format_volume_compact(-12.5), "0");
    assert_eq!(format_volume_compact(f64::NAN), "0");
    assert_eq!(format_volume_compact(f64::INFINITY), "0");
}

#[test]
fn format_volume_compact_keeps_sub_unit_volumes_readable() {
    assert_eq!(format_volume_compact(0.0125), "0.0125");
}

#[test]
fn format_volume_compact_uses_two_decimals_below_a_thousand() {
    assert_eq!(format_volume_compact(5.5), "5.50");
    assert_eq!(format_volume_compact(999.99), "999.99");
}

#[test]
fn format_volume_compact_groups_with_k_m_and_b_suffixes() {
    assert_eq!(format_volume_compact(12_345.0), "12.3K");
    assert_eq!(format_volume_compact(5_000_000.0), "5.00M");
    assert_eq!(format_volume_compact(2_500_000_000.0), "2.50B");
}
