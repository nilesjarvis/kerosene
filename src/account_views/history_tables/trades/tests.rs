use super::*;

#[test]
fn trade_fee_display_marks_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        history_fee_display(&denomination, Some(1.25), None, false),
        "-$1.25"
    );
    assert_eq!(
        history_fee_display(&denomination, None, None, false),
        "Invalid data"
    );
    assert_eq!(history_fee_display(&denomination, None, None, true), "$***");
}

#[test]
fn trade_fee_display_keeps_usdc_fees_as_dollar_amounts() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        history_fee_display(&denomination, Some(1.25), Some("USDC"), false),
        "-$1.25"
    );
    assert_eq!(
        history_fee_display(&denomination, Some(1.25), Some(" "), false),
        "-$1.25"
    );
}

#[test]
fn trade_fee_display_labels_base_token_fees_with_the_fee_token() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    // Spot buys are charged in the base token: 0.02 HYPE is not $0.02.
    assert_eq!(
        history_fee_display(&denomination, Some(0.02), Some("HYPE"), false),
        "-0.02 HYPE"
    );
    // A $50k UBTC buy fee of ~0.0002 UBTC used to render as "-$0.00".
    assert_eq!(
        history_fee_display(&denomination, Some(0.0002), Some("UBTC"), false),
        "-0.0002 UBTC"
    );
    assert_eq!(
        history_fee_display(&denomination, Some(0.00000004), Some("UBTC"), false),
        "-0.00000004 UBTC"
    );
    // Maker rebates are received, not paid.
    assert_eq!(
        history_fee_display(&denomination, Some(-0.01), Some("HYPE"), false),
        "+0.01 HYPE"
    );
    assert_eq!(
        history_fee_display(&denomination, Some(0.02), Some("HYPE"), true),
        "$***"
    );
}
