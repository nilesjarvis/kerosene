use super::format_position_size;

#[test]
fn position_size_formatter_hides_zero_fraction() {
    assert_eq!(format_position_size(1.0), "1");
    assert_eq!(format_position_size(25.0), "25");
}

#[test]
fn position_size_formatter_keeps_nonzero_fraction_precision() {
    assert_eq!(format_position_size(1.25), "1.2500");
    assert_eq!(format_position_size(0.125), "0.1250");
}
