use super::*;

#[test]
fn price_formatter_groups_large_prices() {
    assert_eq!(format_price(1_234.56), "1,234.6");
    assert_eq!(format_price(100_000.0), "100,000.0");
    assert_eq!(format_price(-12_345.67), "-12,345.7");
}

#[test]
fn price_formatter_keeps_existing_precision_bands_below_thousand() {
    assert_eq!(format_price(999.99), "999.99");
    assert_eq!(format_price(0.123456), "0.1235");
}

#[test]
fn grouped_decimal_formatter_keeps_requested_precision() {
    assert_eq!(format_decimal_with_commas(12_345.678_9, 3), "12,345.679");
    assert_eq!(format_decimal_with_commas(12_345.0, 0), "12,345");
}

#[test]
fn signed_percent_formatter_marks_positive_values_and_suppresses_tiny_values() {
    assert_eq!(format_signed_percent_value(1.234), "+1.23%");
    assert_eq!(format_signed_percent_value(-1.234), "-1.23%");
    assert_eq!(format_signed_percent_value(0.004), "0.00%");
    assert_eq!(format_signed_percent_value(-0.004), "0.00%");
}

#[test]
fn two_decimal_display_normalizer_suppresses_values_that_round_to_zero() {
    assert_eq!(normalize_two_decimal_display_value(0.004), 0.0);
    assert_eq!(normalize_two_decimal_display_value(-0.004), 0.0);
    assert_eq!(normalize_two_decimal_display_value(0.005), 0.005);
    assert!(normalize_two_decimal_display_value(f64::NAN).is_nan());
}
