use super::*;

#[test]
fn identity_uses_local_wallet_label_when_available() {
    let wallet_display = WalletDisplay {
        primary: "Local Desk".to_string(),
        secondary: "0xabc0...1234".to_string(),
        has_label: true,
    };

    let name = position_identity(wallet_display);

    assert_eq!(name, "Local Desk");
}

#[test]
fn identity_ignores_api_wallet_labels_without_local_label() {
    let position = sample_position();
    let wallet_display = WalletDisplay {
        primary: "0xabc0...1234".to_string(),
        secondary: position.address.clone(),
        has_label: false,
    };

    let name = position_identity(wallet_display);

    assert_eq!(name, "0xabc0...1234");
}

#[test]
fn numeric_formatters_reject_nonfinite_values() {
    let denomination = DisplayDenominationContext::default();
    assert_eq!(format_usd_number(f64::NAN, &denomination), "-");
    assert_eq!(format_signed_usd(f64::INFINITY, &denomination), "-");
    assert_eq!(format_price_number(0.0, &denomination), "-");
}
