#[test]
fn funding_total_marks_any_invalid_amount_unknown() {
    assert_eq!(super::funding_total(["1", "-0.25"]), Some(0.75));
    assert_eq!(super::funding_total(["1", "bad"]), None);
    assert_eq!(super::funding_total(["1", "NaN"]), None);
}
