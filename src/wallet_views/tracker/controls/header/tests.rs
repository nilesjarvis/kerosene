use super::*;

#[test]
fn tracker_total_formatters_mark_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(money_total_display(&denomination, Some(12.5)), "$12.50");
    assert_eq!(money_total_display(&denomination, None), "Invalid data");
    assert_eq!(
        signed_money_total_display(&denomination, None),
        "Invalid data"
    );
}
