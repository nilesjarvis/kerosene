use super::*;

#[test]
fn wallet_entry_notional_marks_invalid_values() {
    assert_eq!(wallet_entry_notional(Some(12.5)), "$12.50");
    assert_eq!(wallet_entry_notional(Some(0.0)), "-");
    assert_eq!(wallet_entry_notional(None), "Invalid data");
}

#[test]
fn wallet_supplied_amount_distinguishes_missing_and_invalid_values() {
    assert_eq!(wallet_supplied_amount(None, false), "-");
    assert_eq!(wallet_supplied_amount(Some("2.5"), false), "2.500000");
    assert_eq!(wallet_supplied_amount(Some("bad"), false), "Invalid data");
}
