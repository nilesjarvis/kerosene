use super::outcome_probability_text;

#[test]
fn outcome_probability_text_uses_not_available_placeholder_for_invalid_mid() {
    assert_eq!(outcome_probability_text(Some(0.42)), "42.0%");
    assert_eq!(outcome_probability_text(None), "n/a");
    assert_eq!(outcome_probability_text(Some(f64::NAN)), "n/a");
    assert_eq!(outcome_probability_text(Some(f64::INFINITY)), "n/a");
}
