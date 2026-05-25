use super::*;

#[test]
fn funding_rate_display_marks_invalid_values() {
    assert_eq!(funding_rate_display(Some(0.00125)), "0.1250%");
    assert_eq!(funding_rate_display(None), "Invalid data");
}

#[test]
fn funding_amount_display_marks_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        funding_amount_display(&denomination, Some(1.25), false),
        "+$1.2500"
    );
    assert_eq!(
        funding_amount_display(&denomination, Some(-1.25), false),
        "-$1.2500"
    );
    assert_eq!(
        funding_amount_display(&denomination, None, false),
        "Invalid data"
    );
    assert_eq!(funding_amount_display(&denomination, None, true), "$***");
}
