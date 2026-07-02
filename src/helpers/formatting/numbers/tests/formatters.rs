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
fn price_formatter_keeps_significant_figures_across_magnitudes() {
    assert_eq!(format_price(65_000.0), "65,000.0");
    assert_eq!(format_price(42.1), "42.10");
    assert_eq!(format_price(1.234), "1.23");
    assert_eq!(format_price(0.1234), "0.1234");
    assert_eq!(format_price(0.001234), "0.001234");
    assert_eq!(format_price(0.0001234), "0.0001234");
    assert_eq!(format_price(0.00001234), "0.00001234");
}

#[test]
fn price_formatter_preserves_low_priced_spot_mids() {
    // Live spot pairs regularly trade below 0.001; a fixed 4-decimal render
    // showed 0.00005088 as "0.0001" (~2x) and 0.00002314 as "0.0000".
    assert_eq!(format_price(0.00005088), "0.00005088");
    assert_eq!(format_price(0.00002314), "0.00002314");
    assert_eq!(format_price(-0.00005088), "-0.00005088");
    // Decimal places are capped at Hyperliquid's 8-decimal price precision.
    assert_eq!(format_price(0.000000123456), "0.00000012");
    assert_eq!(format_price(0.0), "0.0000");
}

#[test]
fn price_input_formatter_avoids_grouping_and_keeps_precision() {
    assert_eq!(format_price_input(50_000.0), "50000.0000");
    assert_eq!(format_price_input(0.1234), "0.1234");
    assert_eq!(format_price_input(0.0003465), "0.0003465");
    assert_eq!(format_price_input(0.00005088), "0.00005088");
}

#[test]
fn grouped_decimal_formatter_keeps_requested_precision() {
    assert_eq!(format_decimal_with_commas(12_345.678_9, 3), "12,345.679");
    assert_eq!(format_decimal_with_commas(12_345.0, 0), "12,345");
}

#[test]
fn decimal_zero_trimmer_removes_fraction_padding() {
    assert_eq!(trim_decimal_zeros("12,345.6700".to_string()), "12,345.67");
    assert_eq!(trim_decimal_zeros("100.0000".to_string()), "100");
    assert_eq!(trim_decimal_zeros("42".to_string()), "42");
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
