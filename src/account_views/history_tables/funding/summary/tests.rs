use super::*;

#[test]
fn funding_total_display_marks_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        funding_total_display(&denomination, Some(1.25), false),
        "+$1.2500"
    );
    assert_eq!(
        funding_total_display(&denomination, Some(-1.25), false),
        "-$1.2500"
    );
    assert_eq!(
        funding_total_display(&denomination, None, false),
        "Invalid data"
    );
    assert_eq!(funding_total_display(&denomination, None, true), "***");
}
