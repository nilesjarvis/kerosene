use super::*;

#[test]
fn trade_fee_display_marks_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        history_fee_display(&denomination, Some(1.25), false),
        "-$1.25"
    );
    assert_eq!(
        history_fee_display(&denomination, None, false),
        "Invalid data"
    );
    assert_eq!(history_fee_display(&denomination, None, true), "$***");
}
