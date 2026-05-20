use super::*;

#[test]
fn wallet_entry_notional_marks_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(wallet_entry_notional(&denomination, Some(12.5)), "$12.50");
    assert_eq!(wallet_entry_notional(&denomination, Some(0.0)), "-");
    assert_eq!(wallet_entry_notional(&denomination, None), "Invalid data");
}

#[test]
fn wallet_supplied_amount_distinguishes_missing_and_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(wallet_supplied_amount(&denomination, None, false), "-");
    assert_eq!(
        wallet_supplied_amount(&denomination, Some("2.5"), false),
        "2.500000"
    );
    assert_eq!(
        wallet_supplied_amount(&denomination, Some("bad"), false),
        "Invalid data"
    );
}
