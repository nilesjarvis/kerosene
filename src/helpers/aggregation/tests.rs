use super::*;

#[test]
fn add_optional_f64_keeps_poisoned_totals_unknown() {
    let mut total = Some(1.0);
    add_optional_f64(&mut total, Some(2.0));
    assert_eq!(total, Some(3.0));

    add_optional_f64(&mut total, None);
    assert_eq!(total, None);

    add_optional_f64(&mut total, Some(4.0));
    assert_eq!(total, None);
}

#[test]
fn sum_optional_f64_keeps_any_invalid_input_unknown() {
    assert_eq!(sum_optional_f64([Some(1.0), Some(2.0)]), Some(3.0));
    assert_eq!(sum_optional_f64([Some(1.0), None]), None);
}

#[test]
fn positive_percent_change_uses_positive_finite_inputs() {
    assert_eq!(
        positive_percent_change(Some(110.0), Some(100.0)),
        Some(10.0)
    );
    assert_eq!(
        positive_percent_change(Some(75.0), Some(100.0)),
        Some(-25.0)
    );
}

#[test]
fn positive_percent_change_rejects_missing_nonpositive_or_nonfinite_inputs() {
    assert_eq!(positive_percent_change(None, Some(100.0)), None);
    assert_eq!(positive_percent_change(Some(100.0), None), None);
    assert_eq!(positive_percent_change(Some(0.0), Some(100.0)), None);
    assert_eq!(positive_percent_change(Some(100.0), Some(0.0)), None);
    assert_eq!(positive_percent_change(Some(f64::NAN), Some(100.0)), None);
    assert_eq!(
        positive_percent_change(Some(100.0), Some(f64::INFINITY)),
        None
    );
}
