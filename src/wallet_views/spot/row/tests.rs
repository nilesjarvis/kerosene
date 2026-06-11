use super::*;

#[test]
fn wallet_entry_notional_marks_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(wallet_entry_notional(&denomination, Some(12.5)), "$12.50");
    assert_eq!(wallet_entry_notional(&denomination, Some(0.0)), "-");
    assert_eq!(wallet_entry_notional(&denomination, None), "Invalid data");
}

#[test]
fn wallet_spot_amount_formats_outcome_balances_as_whole_contracts() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        wallet_spot_amount(&denomination, "+950", Some(5.0), false),
        "5"
    );
    assert_eq!(
        wallet_spot_amount(&denomination, "+950", Some(12.999999), false),
        "12"
    );
    assert_eq!(
        wallet_spot_amount(&denomination, "+950", None, false),
        "Invalid data"
    );
    assert_eq!(
        wallet_spot_amount(&denomination, "HYPE", Some(2.5), false),
        "2.500000"
    );
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
