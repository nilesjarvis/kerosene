use super::*;

#[test]
fn finite_positive_accepts_only_positive_finite_values() {
    assert_eq!(finite_positive(Some(1.25)), Some(1.25));
    assert_eq!(finite_positive(Some(0.0)), None);
    assert_eq!(finite_positive(Some(-1.0)), None);
    assert_eq!(finite_positive(Some(f64::NAN)), None);
    assert_eq!(finite_positive(Some(f64::INFINITY)), None);
    assert_eq!(finite_positive(None), None);
}

#[test]
fn positive_ratio_keeps_finite_numerator_and_positive_denominator_contract() {
    assert_eq!(positive_ratio(4.0, 2.0), Some(2.0));
    assert_eq!(positive_ratio(-4.0, 2.0), Some(-2.0));
    assert_eq!(positive_ratio(f64::NAN, 2.0), None);
    assert_eq!(positive_ratio(4.0, 0.0), None);
    assert_eq!(positive_ratio(4.0, f64::INFINITY), None);
}
