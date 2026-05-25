use super::*;

#[test]
fn pnl_card_window_defaults_are_privacy_first() {
    let state = position_state("BTC");

    assert_eq!(state.target, PnlCardTarget::Position("BTC".to_string()));
    assert_eq!(state.account_address, test_account());
    assert_eq!(state.display_mode, PnlCardDisplayMode::Both);
    assert_eq!(state.percent_mode, PnlCardPercentMode::Leveraged);
    assert!(state.obscure_prices);
    assert!(!state.show_position_size);
}

#[test]
fn pnl_card_account_binding_rejects_current_account_switch() {
    let state = position_state("BTC");

    assert!(pnl_card_account_matches(
        Some(&test_account().to_uppercase()),
        &state
    ));
    assert!(!pnl_card_account_matches(Some(&other_account()), &state));
    assert!(!pnl_card_account_matches(None, &state));
}
