use super::*;

#[test]
fn book_tick_validation_accepts_only_positive_finite_values() {
    assert!(valid_book_tick_size(0.01));
    assert!(!valid_book_tick_size(0.0));
    assert!(!valid_book_tick_size(-0.01));
    assert!(!valid_book_tick_size(f64::NAN));
    assert!(!valid_book_tick_size(f64::INFINITY));
}

#[test]
fn tick_helpers_fallback_for_invalid_prices_or_ticks() {
    assert_eq!(default_tick_for_price(f64::NAN), 0.01);
    assert_eq!(default_tick_for_price(f64::INFINITY), 0.01);
    assert_eq!(compute_sigfigs(f64::NAN, 100.0), (None, None));
    assert_eq!(compute_sigfigs(0.01, f64::NAN), (None, None));
    assert_eq!(format_tick(f64::NAN), "-");
}
