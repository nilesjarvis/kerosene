use super::*;

#[test]
fn formats_hype_amount_with_selected_denomination_notional() {
    let denomination = DisplayDenominationContext::usd();

    assert_eq!(
        format_hype_amount_with_notional(150_000_000, Some(40.0), &denomination),
        "1.5 HYPE ($60.00)"
    );
}

#[test]
fn formats_hype_amount_with_missing_notional_price() {
    let denomination = DisplayDenominationContext::usd();

    assert_eq!(
        format_hype_amount_with_notional(150_000_000, None, &denomination),
        "1.5 HYPE (n/a)"
    );
}

#[test]
fn wallet_identity_uses_label_and_tooltip_keeps_address() {
    let display = WalletDisplay {
        primary: "Market Maker Alpha".to_string(),
        secondary: "0x1234...abcd".to_string(),
        has_label: true,
    };

    assert_eq!(hype_unstaking_wallet_label(&display), "Market Maker Alpha");
    assert_eq!(
        hype_unstaking_wallet_tooltip(&display, "0x1234567890abcdef1234567890abcdef12345678"),
        "Market Maker Alpha (0x1234567890abcdef1234567890abcdef12345678)"
    );
}

#[test]
fn wallet_label_preserves_the_26_character_limit_for_unicode() {
    for (primary, expected) in [
        (String::new(), String::new()),
        ("a".repeat(26), "a".repeat(26)),
        ("a".repeat(27), format!("{}...", "a".repeat(23))),
        ("界".repeat(26), "界".repeat(26)),
        ("界".repeat(27), format!("{}...", "界".repeat(23))),
        ("a界".repeat(14), format!("{}a...", "a界".repeat(11))),
    ] {
        let display = WalletDisplay {
            primary,
            secondary: String::new(),
            has_label: true,
        };
        assert_eq!(hype_unstaking_wallet_label(&display), expected);
    }
}
